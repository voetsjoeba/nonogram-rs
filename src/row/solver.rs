// vim: set ai et ts=4 sts=4 sw=4:
use std::fmt;
use std::ops::Range;
use std::convert::{TryInto, TryFrom};
use std::cmp::{min, max};
use std::rc::{Rc, Weak};
use std::cell::{Ref, RefMut, RefCell};
use std::collections::HashSet;
use super::{Row, Run, DirectionalSequence};
use super::super::util::{Direction, Direction::*, vec_remove_item};
use super::super::grid::{Grid, Square, SquareStatus::{CrossedOut, FilledIn, Unknown},
                         Changes, Change, Error, HasGridLocation};

impl Row {

    pub fn update_possible_run_placements(&mut self)
    {
        // for each run in this row, calculates the possible placements of the run within the row,
        // taking the current state of the row into account (i.e. crossed out squares, filled in squares, etc).

        // a run of length L can be placed at position S, creating a range we'll denote as S..E,
        // if and only if:
        // - none of the squares in the range S..E are crossed out
        // - none of the squares in the range S..E are already marked as belonging to another run
        // - the range S..E is not directly adjacent to any square that is filled in
        // - the starting position S is no smaller than the previous run's earliest ending position + 1 (or 0 if there is no previous run)
        // - the ending position E is no bigger than the next run's latest starting position - 1 (or row length if there is no next run)
        // - if this is the first run, there cannot be any filled in squares to our left that we don't contain
        //   (and analogously for the last run).


        // the valid positions of a run depend on those of the runs that come before AND after it, but we can
        // only iterate on direction at a time.
        // so we'll work in two stages:
        //  1) L -> R scan, determining the possible run placements looking only at those of the runs before it
        //  2) R -> L scan, dropping possible run placements that:
        //       * infringe on the requirement of having to end to end before the following run's latest starting position - 1.
        //       * infringe on the requirement of having to contain ALL squares assigned to this run in the row.

        // 1) L -> R scan
        //println!("  update_possible_run_placements: L -> R scan");
        for run_idx in 0..self.runs.len()
        {
            let run = &self.runs[run_idx];
            let len = run.length;

            if run.is_completed() {
                // nothing to do
                assert!(run.possible_placements.len() == 1);
                continue;
            }
            //println!("    evaluating run #{} (len {})", run_idx, len);

            let mut possible_placements = Vec::<Range<usize>>::new();

            // what is the previous run's earliest ending position (if there is such a run)?
            let mut prev_run_earliest_end: isize = -1;
            if run_idx > 0 {
                let prev_run = &self.runs[run_idx-1];
                prev_run_earliest_end = prev_run.possible_placements[0].end.try_into().unwrap(); // [0] should always exist, was computed in one of the previous iterations
            }

            let assigned_squares = (0..self.length).filter(|&pos| self.get_square(pos).has_run_assigned(run))
                                                   .collect::<Vec<_>>();
            let filled_squares = (0..self.length).filter(|&pos| self.get_square(pos).get_status() == FilledIn)
                                                 .collect::<Vec<_>>();

            let scan_start: usize = usize::try_from(prev_run_earliest_end + 1).unwrap();
            let scan_end: usize = self.length - len + 1;
            //println!("      prev_run_earliest_end = {}, scan_start = {}, scan_end = {}", prev_run_earliest_end, scan_start, scan_end);

            #[allow(unused_parens)]
            for s in scan_start .. scan_end
            {
                let range = (s .. s+len);
                let any_crossed_out      = range.clone().any(|pos| self.get_square(pos).get_status() == CrossedOut);
                let any_belongs_to_other = range.clone().any(|pos| match self.get_square(pos).get_run_index(self.direction) {
                                                                      Some(x) => x != run_idx,
                                                                      None    => false,
                                                                   });
                let mut any_adj_sq_filled_in = false;
                if range.start > 0 {
                    any_adj_sq_filled_in = any_adj_sq_filled_in || self.get_square(range.start-1).get_status() == FilledIn;
                }
                if range.end < self.length { // range.end is exclusive, so following square is at exactly range.end
                    any_adj_sq_filled_in = any_adj_sq_filled_in || self.get_square(range.end).get_status() == FilledIn;
                }

                let contains_first_assigned = match assigned_squares.first() {
                    Some(pos) => range.contains(pos),
                    None      => true,
                };
                let contains_last_assigned = match assigned_squares.last() {
                    Some(pos) => range.contains(pos),
                    None      => true,
                };
                // if this is the first run, we can't be positioned beyond the first filled square (if any).
                let beyond_first_filled = run_idx == 0 && match filled_squares.first() {
                    Some(&pos) => range.start > pos,
                    None       => false,
                };
                // analogously for the last run and the last filled square (if any)
                let beyond_last_filled = run_idx == self.runs.len()-1 && match filled_squares.last() {
                    Some(&pos) => range.end <= pos,
                    None       => false,
                };

                if    !any_crossed_out
                   && !any_belongs_to_other
                   && !any_adj_sq_filled_in
                   && contains_first_assigned
                   && contains_last_assigned
                   && !beyond_first_filled
                   && !beyond_last_filled
                {
                    // possible placement, add it
                    possible_placements.push(range);
                }
            }

            //println!("      possible placements (ignoring next runs): {}", possible_placements.iter()
            //                                                                                  .map(|range| format!("[{},{}]", range.start, range.end-1))
            //                                                                                  .collect::<Vec<_>>()
            //                                                                                  .join(", "));
            let run: &mut Run = &mut self.runs[run_idx];
            run.possible_placements = possible_placements;
        }

        // 2) R -> L scan
        //println!("");
        //println!("  update_possible_run_placements: R -> L scan");
        for run_idx in (0..(self.runs.len()-1)).rev() {
            //println!("    evaluating run #{} (len {})", run_idx, len);
            let run = &self.runs[run_idx];
            if run.is_completed() {
                continue; // nothing to do
            }

            let next_run = &self.runs[run_idx+1];
            let next_run_latest_start: usize = next_run.possible_placements.last().unwrap().start.try_into().unwrap();
            //println!("      next_run_latest_start (run #{}, {}) = {}", next_run.index, next_run.length, next_run_latest_start);

            // drop placements that don't respect the condition that this run's end position
            // must be no greater than the next one's latest start position - 1
            let run = &mut self.runs[run_idx];
            run.possible_placements.retain(|range| range.end <= next_run_latest_start-1);

            //println!("      corrected ranges: {}", run.possible_placements.iter()
            //                                                              .map(|range| format!("[{},{}]", range.start, range.end-1))
            //                                                              .collect::<Vec<_>>()
            //                                                              .join(", "));

        }


        // make sure all runs received at least one possible placement, otherwise something's wrong
        for run in &self.runs {
            if run.possible_placements.len() == 0 {
                panic!("Inconsistency: no possible placements found for {} run #{} of length {} in {} row {}",
                       self.direction,
                       run.index,
                       run.length,
                       self.direction,
                       self.index);
            }
        }
    }

    pub fn infer_status_assignments(&mut self) -> Result<Changes, Error>
    {
        // look at the possible placements of each run:
        // - if there are squares that are part of all of them, then those must necessarily be filled in and assigned to that run.
        // - if there's only one possible placement, then we can place it at that position and mark the run as completed
        let mut changes = Vec::<Change>::new();
        for run in &mut self.runs
        {
            if run.is_completed() { continue; } // nothing to do
            for pos in 0..self.length {
                let mut square: RefMut<Square> = run.get_square_mut(pos);
                if run.possible_placements.iter().all(|range| range.contains(&pos))
                {
                    if let Some(change) = square.set_status(FilledIn)? {
                        changes.push(Change::from(change));
                    }
                    if let Some(change) = square.assign_run(run)? {
                        changes.push(Change::from(change));
                    }
                }
            }

            if run.possible_placements.len() == 1 {
                let range = run.possible_placements[0].clone(); // clone to avoid immutable borrow through mut ref
                changes.extend(run.complete(range.start)?);
            }
        }

		// conversely, look at all the squares in this row:
        // - if there are squares that aren't part of any run, then those must necessarily be crossed out
        for pos in 0..self.length {
            let part_of_any_run = self.runs.iter()
                                           .any(|run| run.possible_placements.iter()
                                                                             .any(|range| range.contains(&pos)));
            if !part_of_any_run {
                if let Some(change) = self.get_square_mut(pos).set_status(CrossedOut)? {
                    changes.push(Change::from(change));
                }
            }
        }

        //if changes.len() > 0 {
        //    println!("fill_overlap completed successfully; changes are:");
        //    for c in changes.iter() {
        //        println!("  {}", c);
        //    }
        //}

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
        // after assigning a run to a square, update that run's set of possible placements as well,
        // since those might have tightened up now.
        for seq_idx in 0..filled_sequences.len()
        {
            let seq = &filled_sequences[seq_idx];
            let possible_runs = self.possible_runs_for_sequence(seq);

            if possible_runs.len() == 0 {
                panic!("Inconsistency: no run found that can encompass the sequence of filled squares [{}, {}] in {} row {}", seq.start, seq.end-1, self.direction, self.index);
            }
            else if possible_runs.len() == 1 {
                // only one run could possibly encompass this sequence; assign it to each square
                let run = &self.runs[possible_runs[0]];

                for x in seq.start..seq.end {
                    if let Some(change) = self.get_square_mut(x).assign_run(run)? {
                        println!("  infer_run_assignments: found singular run assignment for sequence [{}, {}]: run {} (len {})", seq.start, seq.end-1, run.index, run.length);
                        changes.push(Change::from(change));
                    }
                }

                // on the next iteration, update_possible_run_placements will pick up on the fact that this square
                // got a run assigned to it, and update its possible placements accordingly.
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
                // against the edges of the containing field to find additional squares to be filled in.
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
                let field = self.get_fields().into_iter()
                                             .filter(|field| field.contains(&seq.start))
                                             .next()
                                             .expect("");

                let clamped_leftmost_start = max(seq.start - min_length + 1, field.start);
                let clamped_rightmost_end  = min(seq.start + min_length,     field.end);

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
