// vim: set ai et ts=4 sts=4 sw=4:
use std::fmt;
use std::ops::Range;
use std::convert::{TryInto, TryFrom};
use std::cmp::{min, max};
use std::rc::{Rc, Weak};
use std::cell::{Ref, RefMut, RefCell};
use std::collections::HashSet;
use super::{Row, Field, Run, DirectionalSequence};
use super::super::util::{Direction, Direction::*};
use super::super::grid::{Grid, Square, SquareStatus::{CrossedOut, FilledIn}, Changes, Change, Error, HasGridLocation};

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
        assert!(self.fields.len() > 0, "No fields exist in this row!");

        // L -> R scan: for each run in sequence, find the first field that can contain it
        let mut current_field: usize = 0;
        let mut candidate_position: usize = 0;
        for run in &mut self.runs
        {
            run.min_start = None;
            while current_field < self.fields.len() {
                let field: &Field = &self.fields[current_field];
                // we started evaluating a new field; skip ahead to the start of this field
                candidate_position = max(candidate_position, field.offset);
                let shift = candidate_position - field.offset; // relative offset within this field
                if ! field.run_lfits_at(run, shift) {
                    // not enough space (or space left) in this field, try the next one
                    current_field += 1;
                    continue;
                }

                // field has enough space to contain the run; place this run and start evaluating the next
                // one, but stay in this field (it may still have space left to contain the next one)
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
        let mut candidate_end_position: isize = (self.length).try_into().unwrap();

        for run in self.runs.iter_mut().rev()
        {
            run.max_start = None;
            //for field in self.fields.iter().rev() {
            while current_field >= 0 {
                let field: &Field = &self.fields[usize::try_from(current_field).unwrap()];
                let field_end: isize = (field.offset+field.length).try_into().unwrap();

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
        let filled_ranges = self._ranges_of(|s| s.get_status() == FilledIn)
                                .into_iter().collect::<Vec<_>>();

        // look through this row for contiguous ("attached") sequences of filled squares;
        // for each one found, see whether it falls within any of the runs' min_start and max_start
        // ranges (should always be at least one).
        // because these are attached sequences, if ANY of the squares within the sequence falls within the
        // range of only a single run, then the whole sequence must be part of that run and we can assign it.
        for range in filled_ranges
        {
            // find runs in this row where run.min_start <= range[0] <= run.max_start
            //let possible_runs: Vec<Vec<&Run>> =
            //    range.map(|x| self.runs.iter()
            //                           .filter(|r| r.is_possible_start_position(x))
            //                           .collect::<Vec<_>>())
            //         .collect();

            let mut single_run: Option<&Run> = None;
            for x in range.start..range.end {
                let possible_runs: Vec<&Run> = self.runs.iter()
                                                        .filter(|r| r.is_possible_start_position(x))
                                                        .collect::<Vec<_>>();
                if possible_runs.len() == 0 {
                    panic!("inconsistency: no run found that can encompass the sequence of filled squares in {} row {} between indices {} and {} (incl.)", self.direction, self.index, range.start, range.end);
                }
                if possible_runs.len() == 1 {
                    // only one run could possibly encompass this sequence of filled squares; assign it to all of them
                    single_run = Some(possible_runs[0]);
                    break;
                }
            }
            if let Some(run) = single_run {
                for i in range.start..range.end {
                    if let Some(change) = self.get_square_mut(i).assign_run(run)? {
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
        let filled_ranges = self._ranges_of(|s| s.get_status() == FilledIn)
                                .into_iter().collect::<Vec<_>>();

        for range in filled_ranges
        {
            let mut unique_runs = HashSet::<usize>::new();
            for i in range.start..range.end {
                if let Some(x) = self.get_square(i).get_run_index(self.direction) {
                    unique_runs.insert(x);
                }
            }

            if unique_runs.len() > 1 {
                // found more than one run in contiguous sequence of squares; inconsistency
                panic!("Found {} different runs in contiguous sequence of {} squares in {} row {}",
                    unique_runs.len(), range.len(), self.direction, self.index);
            }
            if unique_runs.len() == 1 {
                // assign run to all squares in this sequence
                let run_index: usize = *unique_runs.iter().next().unwrap();
                let run: &mut Run = &mut self.runs[run_index];

                if run.is_completed() { continue; }

                for i in range.start..range.end {
                    if let Some(change) = run.get_square_mut(i).assign_run(run)? {
                        changes.push(Change::from(change));
                    }
                }
                // if the range has the same length as the run, then we've found a completed run
                if range.len() == run.length {
                    //println!("found new completed run of length {} in {} row {} at offset {}", run.length, self.direction, run.get_row_index(), range.start);
                    changes.extend(run.complete(range.start)?);
                }
            }
        }

        Ok(changes)
    }

    pub fn check_completed(&mut self) -> Result<Changes, Error> {
        // if all runs in this row have been completed, clear out any remaining squares
        let mut changes = Vec::<Change>::new();
        if self.runs.iter().all(|r| r.is_completed())
        {
            for x in 0..self.length {
                let mut square: RefMut<Square> = self.get_square_mut(x);
                if square.get_status() != FilledIn {
                    if let Some(change) = square.set_status(CrossedOut)? {
                        changes.push(Change::from(change));
                    }
                }
            }
            self.completed = true;
        }

        Ok(changes)
    }

}
