// vim: set ai et ts=4 sts=4 sw=4:
use std::fmt;
use std::ops::Range;
use std::convert::{TryInto, TryFrom};
use std::cmp::{min, max};
use std::rc::{Rc, Weak};
use std::cell::{Ref, RefMut, RefCell};
use std::collections::HashSet;
use super::{Row, Field, Run, DirectionalSequence};
use super::super::util::{Direction, Direction::*, vec_remove_item};
use super::super::grid::{Grid, Square, SquareStatus::{CrossedOut, FilledIn, Unknown},
                         Changes, Change, Error, HasGridLocation};

impl Row {
    fn _ranges_of<P>(&self, pred: P) -> Vec<Range<usize>>
        where P: Fn(Ref<Square>) -> bool
    {
        let mut result = Vec::<Range<usize>>::new();
        let mut x: usize = 0;
        while x < self.length {
            // skip past squares for which the predicate does not hold
            while x < self.length && !pred(self.get_square(x)) {
                x += 1;
            }
            if x >= self.length { break; }

            // skip past squares for which the predicate does hold
            let range_start = x;
            x += 1; // we already tested the predicate on x at the end of the previous loop
            while x < self.length && pred(self.get_square(x)) {
                x += 1;
            }
            let range_end = x;
            result.push(range_start..range_end);

            x += 1;
        }
        result
    }

    pub fn recalculate_fields(&mut self)
    {
        self.fields = self._ranges_of(|s| s.get_status() != CrossedOut)
                          .into_iter()
                          .map(|range| self.make_field(range.start, range.end-range.start))
                          .collect();
    }

    pub fn update_run_bounds(&mut self)
    {
        // update the min_start and max_start bounds of each run, given the current set of
        // fields in the row.
        // NOTE: each run might already have an existing min_start and max_value value from previous
        // solver iterations; we should only further refine them, not reset them.
        assert!(self.fields.len() > 0, "No fields exist in this row!");

        // L -> R scan: for each run in sequence, find the first field that can contain it
        let mut current_field: usize = 0;
        let mut candidate_position: usize = 0;
        for run in &mut self.runs
        {
            // start off from previously set min_start, if any
            if let Some(existing_min_start) = run.min_start {
                candidate_position = max(candidate_position, existing_min_start);
            }
            while current_field < self.fields.len() {
                let field: &Field = &self.fields[current_field];
                // we started evaluating a new field; skip ahead to the start of this field if necessary
                candidate_position = max(candidate_position, field.offset);
                let shift = candidate_position - field.offset; // are we at a relative offset within this field?
                if ! field.run_lfits_at(run, shift) {
                    // not enough space (or space left) in this field, move on to the next one
                    current_field += 1;
                    continue;
                }

                // field has enough space to contain the run; place this run and start evaluating the next
                // one, but stay in this field (it may still have space left to contain the next run)
                run.min_start = Some(candidate_position);
                candidate_position += run.length + 1;

                break;
            }

            if current_field >= self.fields.len() {
                // no position found for this run, so we can stop here because
                // we also won't found positions for any runs after this one
                break;
                //panic!("");
            }
        }

        // R -> L scan: for each run in sequence, find the last field that can contain it
        let mut current_field: isize = (self.fields.len() - 1).try_into().unwrap();
        let mut candidate_end_position: isize = (self.length).try_into().unwrap(); // exclusive!

        for run in self.runs.iter_mut().rev()
        {
            // start off from previously set max_start, if any
            if let Some(existing_max_start) = run.max_start {
                candidate_end_position = min(candidate_end_position,
                                             isize::try_from(existing_max_start).unwrap() + isize::try_from(run.length).unwrap());
            }
            while current_field >= 0 {
                let field: &Field = &self.fields[usize::try_from(current_field).unwrap()];
                let field_end: isize = (field.offset+field.length).try_into().unwrap(); // exclusive

                candidate_end_position = min(candidate_end_position, field_end);
                let rshift = field_end - candidate_end_position;

                if ! field.run_rfits_at(run, rshift.try_into().unwrap()) {
                    // not enough space (or space left) in this field, try the next one
                    current_field -= 1;
                    continue
                }

                // field has enough space to contain the run; place this run and start evaluating the next
                // one, but stay in this field (it may still have space left to contain the next one)
                run.max_start = Some(usize::try_from(candidate_end_position).unwrap() - run.length);
                candidate_end_position -= isize::try_from(run.length+1).unwrap();
                break;
            }
            if current_field < 0 {
                // no position found for this run, so we can stop here because
                // we also won't found positions for any runs after this one
                break;
                //panic!("");
            }
        }

        // check if all runs received a possible placement
        for (i, run) in self.runs.iter().enumerate() {
            assert!(run.max_start >= run.min_start);
            if let None = run.min_start {
                // no possible placement found for this run
                panic!("No leftmost placement found for {} run #{} of length {} in {} {}",
                       self.direction,
                       i+1,
                       run.length,
                       match self.direction {
                           Horizontal => "row",
                           Vertical   => "col",
                       },
                       self.index);
            }
            if let None = run.max_start {
                // no possible placement found for this run
                panic!("No rightmost placement found for {} run #{} of length {} in {} {}",
                       self.direction,
                       i+1,
                       run.length,
                       match self.direction {
                           Horizontal => "row",
                           Vertical   => "col",
                       },
                       self.index);
            }
        }
    }
    pub fn fill_overlap(&mut self) -> Result<Changes, Error>
    {
        let mut changes = Vec::<Change>::new();
        for run in &mut self.runs
        {
            let max_start = run.max_start.unwrap();
            let min_start = run.min_start.unwrap();
            let diff = max_start - min_start;

            if diff < run.length {
                // found overlap
                let overlap_start = max_start;
                let overlap_len   = run.length - diff;
                for i in 0..overlap_len {
                    let mut square: RefMut<Square> = run.get_square_mut(overlap_start+i);
                    if let Some(change) = square.set_status(FilledIn)? {
                        changes.push(Change::from(change));
                    }
                    if let Some(change) = square.assign_run(run)? {
                        changes.push(Change::from(change));
                    }
                }
                if diff == 0 {
                    changes.extend(run.complete(overlap_start)?);
                }
            }
        }
        /*if changes.len() > 0 {
            println!("fill_overlap completed successfully; changes are:");
            for c in changes.iter() {
                println!("  {}", c);
            }
        }*/

        Ok(changes)
    }
    pub fn infer_run_assignments(&mut self) -> Result<Changes, Error>
    {
        let mut changes = Vec::<Change>::new();
        let filled_sequences = self._ranges_of(|s| s.get_status() == FilledIn)
                                   .into_iter().collect::<Vec<_>>();

        // look through this row for contiguous ("attached") sequences of filled squares;
        // for each one found, determine which runs that sequence could be part of:
        //  - if there's only one possible run, then we can unambiguously assign it
        //  - if there are multiple runs, but they're all of the same length as the sequence, then
        //    we can confirm the length of the sequence and cross out squares in front and behind of it
        //
        // after assigning a run to a square, update that run's min_start and max_start positions as well,
        // since those might have tightened up now.
        for seq_idx in 0..filled_sequences.len()
        {
            let seq = &filled_sequences[seq_idx];
            let mut possible_runs = self.possible_runs_for_sequence(seq);

            // if this is not the first filled sequence (i.e. there are one or more others to our left in the same field),
            // then we can exclude the possibility of the first run in the row if joining up to the
            // leftmost sequence would exceed the length of the run.
            // (and analogously for the last run).

            // Motivating example:
            //              0 1 2 3 4   5 6 7 8 9   A B C D E
            //     3 9    [   . . . . │ X . . X . │ . X X X . │ . . . . . ]
            //
            //  run  3: min_start =  1, max_start =  7                                 
            //  run  9: min_start =  5, max_start = 11
            //
            // In this scenario, the sequence [8,8] cannot be assigned to run 3, because that's the first run of the row,
            // and it would hence necessarily have to join up to the filled square further to its left, but that
            // would create a sequence of length 4. The only remaining option is therefore run 9.
            if seq_idx > 0 {
                let joined_to_first_seq = filled_sequences[0].start .. seq.end;
                if joined_to_first_seq.len() > self.runs[0].length {
                    if let Some(_) = vec_remove_item(&mut possible_runs, &0usize) {
                        println!("  infer_run_assignments: removing the possibility of run {} (length {}) for the sequence [{}, {}]: would require joining up with the earlier sequence [{}, {}] for a resulting size of {}, exceeding the run's length",
                            0, self.runs[0].length, seq.start, seq.end-1, filled_sequences[0].start, filled_sequences[0].end-1, joined_to_first_seq.len());
                    }
                }
            }
            if seq_idx < filled_sequences.len()-1 {
                let joined_to_last_seq = seq.start .. filled_sequences[filled_sequences.len()-1].end;
                if joined_to_last_seq.len() > self.runs[self.runs.len()-1].length {
                    if let Some(_) = vec_remove_item(&mut possible_runs, &(self.runs.len()-1)) {
                        println!("  infer_run_assignments: removing the possibility of run {} (length {}) for the sequence [{}, {}]: would require joining up with the later sequence [{}, {}] for a resulting size of {}, exceeding the run's length",
                            self.runs.len()-1, self.runs[self.runs.len()-1].length, seq.start, seq.end-1, filled_sequences[filled_sequences.len()-1].start, filled_sequences[filled_sequences.len()-1].end-1, joined_to_last_seq.len());
                    }
                }
            }

            if possible_runs.len() == 0 {
                panic!("Inconsistency: no run found that can encompass the sequence of filled squares [{}, {}] in {} row {}", seq.start, seq.end-1, self.direction, self.index);
            }
            else if possible_runs.len() == 1 {
                // only one run could possibly encompass this sequence; assign it to each square
                let run = &self.runs[possible_runs[0]];
                println!("  infer_run_assignments: found singular run assignment for sequence [{}, {}]: run {} (len {})", seq.start, seq.end-1, run.index, run.length);

                for x in seq.start..seq.end {
                    if let Some(change) = self.get_square_mut(x).assign_run(run)? {
                        changes.push(Change::from(change));
                    }
                }

                // update min_start and max_start of the run given that we've now potentially found new squares assigned
                // to it; update_run_bounds and fill_overlap will pick up these new values in the next iteration(s).
                // (note: have to dip into signed arithmetic for a second because the second argument to max() may be negative
                let run = &mut self.runs[possible_runs[0]];
                run.min_start = Some(usize::try_from(
                    max(isize::try_from(run.min_start.unwrap()).unwrap(),
                        isize::try_from(seq.end).unwrap() - isize::try_from(run.length).unwrap())
                ).unwrap());
                run.max_start = Some(min(run.max_start.unwrap(), seq.start));
            }
            else {
                // ok, we couldn't identify an exact run; see if there's anything else we can determine with the
                // information we have.

                // if all possible runs for this sequence are of the same length that the sequence already has,
                // then we can at least confirm its placement despite not knowing exactly which one it is yet.
                if possible_runs.iter().all(|&r| self.runs[r].length == seq.len()) {
                    //println!("all possible runs that might contain the sequence [{}, {}] are of the same length: {}", seq.start, seq.end-1, seq.len());
                    // pick any run (doesn't matter which one, they're all the same length), pretend it will be placed
                    // at this sequence's position, and cross out the squares directly in front of and behind it.
                    changes.extend(self.runs[possible_runs[0]].delineate_at(seq.start)?);
                }

                // if all possible runs are of a certain minimum length, we can 'bounce' that length
                // against the edges of the field to find additional squares to be filled in.
                //
                // example:
                //              0 1 2 3 4   5 6 7 8 9   A B C D E
                //   1 2 2 2  [ X   . . . │ . . . . . │ .   X . . │ . . . . . ]
                //
                // in this scenario, the square at position D can be marked as filled in, because all possible
                // runs that can contain it are of size >= 2.
                let min_length = possible_runs.iter().map(|&r| self.runs[r].length).min().unwrap();
                if min_length > seq.len() {
                    println!("  infer_run_assignments: all possible runs for sequence [{}, {}] are of length at least {}; marking additional squares away from field edges as filled in (where applicable)", seq.start, seq.end-1, min_length);
                }
                let field = self.fields.iter()
                                       .filter(|field| field.contains(seq.start))
                                       .next()
                                       .expect(""); // TODO: code duplication from possible_runs_for_sequence

                let clamped_leftmost_start = max(seq.start - min_length + 1, field.range().start);
                let clamped_rightmost_end  = min(seq.start + min_length,     field.range().end);

                let clamped_leftmost_range = clamped_leftmost_start .. (clamped_leftmost_start + min_length);
                let clamped_rightmost_range = (clamped_rightmost_end - min_length) .. clamped_rightmost_end;

                // fill in from seq.start to clamped_leftmost_range.end
                //              clamped_rightmost_range.start to seq.end
                for x in seq.start .. clamped_leftmost_range.end {
                    if let Some(change) = self.get_square_mut(x).set_status(FilledIn)? {
                        changes.push(Change::from(change));
                    }
                }
                for x in clamped_rightmost_range.start .. seq.end {
                    if let Some(change) = self.get_square_mut(x).set_status(FilledIn)? {
                        changes.push(Change::from(change));
                    }
                }

            }
        }

        Ok(changes)
    }

    pub fn infer_status_assignments(&mut self) -> Result<Changes, Error>
    {
        // kind of the converse of infer_run_assignments: cross out squares that don't fall within the potential range
        // of any of the runs.
        let mut changes = Vec::<Change>::new();
        for x in 0..self.length {
            if !self.runs.iter().any(|r| r.might_contain_position(x)) {
                if let Some(change) = self.get_square_mut(x).set_status(CrossedOut)? {
                    changes.push(Change::from(change));
                }
            }
        }

        // look at single unknown squares inbetween sequences of filled in ones; if filling them in would create
        // a sequence that can't exist at that position, then that square has to be crossed out.
        //
        // Example:
        //              0 1 2 3 4   5 6 7 8 9   A B C D E
        //   1 2 4    [ . .     X │ . X . . . │ . X . . . ]
        //
        //  run  1: min_start =  0, max_start =  6
        //  run  2: min_start =  4, max_start =  8
        //  run  4: min_start =  8, max_start = 11
        //
        // In this scenario, the square at position 5 cannot be filled in, because that would create a sequence
        // of length 3 outside of the range of any run of length >= 3. Another reason could be that the field
        // that the sequence lives in doesn't have enough space to contain any run length >= 3.
        let filled_sequences = self._ranges_of(|s| s.get_status() == FilledIn)
                                   .into_iter().collect::<Vec<_>>();
        let gap_squares = (1..(self.length-1)).filter(|&x| self.get_square(x-1).get_status() == FilledIn
                                                           && self.get_square(x).get_status() == Unknown
                                                           && self.get_square(x+1).get_status() == FilledIn)
                                              .collect::<Vec<_>>();
        for gap_position in gap_squares
        {
            println!("  infer_status_assignments: found gap square at position {}", gap_position);
            let filled_seq_left  = filled_sequences.iter().filter(|r| r.end   == gap_position).next().expect("");
            let filled_seq_right = filled_sequences.iter().filter(|r| r.start == gap_position+1).next().expect("");
            let joined_seq = filled_seq_left.start .. filled_seq_right.end;

            // could a filled in sequence [filled_seq_left.start, filled_seq_right.end[ exist at this position?
            // i.e., is there a run of length >= joined_len that might contain any of the squares in that range,
            // and that could be placed in this field?
            let possible_runs = self.possible_runs_for_sequence(&joined_seq);
            if possible_runs.len() == 0 {
                println!("  infer_status_assignments: no run can contain joined sequence of len {} if this square were to be filled in; crossing it out", joined_seq.len());
                // no runs can contain the joined sequence if we filled in this gap square, so it has to be crossed out.
                if let Some(change) = self.get_square_mut(gap_position).set_status(CrossedOut)? {
                    changes.push(Change::from(change));
                }
            }
        }

        Ok(changes)
    }

    pub fn check_completed_runs(&mut self) -> Result<Changes, Error>
    {
        // scan for attached sequences of filled in squares; for each sequence,
        // if any of the squares have a run assigned, then expand that run to all other squares
        // in the sequence. also, if the length of the sequence is the same as that of the run
        // it was assigned, then the run is complete.
        let mut changes = Vec::<Change>::new();
        let filled_sequences = self._ranges_of(|s| s.get_status() == FilledIn)
                                   .into_iter().collect::<Vec<_>>();

        for seq in filled_sequences
        {
            let mut unique_runs = HashSet::<usize>::new();
            for i in seq.start..seq.end {
                if let Some(x) = self.get_square(i).get_run_index(self.direction) {
                    unique_runs.insert(x);
                }
            }

            if unique_runs.len() > 1 {
                // found more than one run in contiguous sequence of squares; inconsistency
                panic!("Found {} different runs in contiguous sequence of {} squares in {} row {}",
                    unique_runs.len(), seq.len(), self.direction, self.index);
            }
            if unique_runs.len() == 1 {
                // assign run to all squares in this sequence
                let run_index: usize = *unique_runs.iter().next().unwrap();
                let run: &mut Run = &mut self.runs[run_index];

                if run.is_completed() { continue; }

                for i in seq.start..seq.end {
                    if let Some(change) = run.get_square_mut(i).assign_run(run)? {
                        changes.push(Change::from(change));
                    }
                }
                // if the sequence has the same length as the run, then we've found a completed run
                if seq.len() == run.length {
                    //println!("found new completed run of length {} in {} row {} at offset {}", run.length, self.direction, run.get_row_index(), range.start);
                    changes.extend(run.complete(seq.start)?);
                }
            }
        }

        Ok(changes)
    }

    #[allow(unused_parens)]
    pub fn check_completed(&mut self) -> Result<Changes, Error> {
        // if all runs in this row have been completed, clear out any remaining squares
        // (also handles cases where the row is empty or only has 0-length runs)
        let mut changes = Vec::<Change>::new();
        let is_trivially_empty: bool = self.is_trivially_empty();

        if is_trivially_empty || self.runs.iter().all(|r| r.is_completed())
        {
            for x in 0..self.length {
                let mut square: RefMut<Square> = self.get_square_mut(x);
                // if this row is empty, cross out everything; otherwise, only cross out whatever wasn't already crossed out
                if is_trivially_empty || square.get_status() != FilledIn {
                    if let Some(change) = square.set_status(CrossedOut)? {
                        changes.push(Change::from(change));
                    }
                }
            }
            self.completed = true;
        }

        // just for proper visual coloring when printing out a puzzle with 0-length runs, mark all 0-runs completed
        if is_trivially_empty {
            for run in &mut self.runs {
                assert!(run.length == 0);
                run.completed = true;
            }
        }

        Ok(changes)
    }

}
