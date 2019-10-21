// vim: set ai et ts=4 sw=4 sts=4:
//use std::iter::Iterator;
use std::fmt;
use std::ops::Range;
use std::convert::{TryInto, TryFrom};
use std::cmp::{min, max};
use std::rc::{Rc, Weak};
use std::cell::{Ref, RefMut, RefCell};
use super::util::{Direction, Direction::*};
use super::grid::{Grid, Square, SquareStatus::{CrossedOut, FilledIn}};

#[derive(Debug)]
pub struct Row {
    pub direction:  Direction,
    pub index:      usize,
    pub length:     usize,
    pub runs:       Vec<Run>,
    pub fields:     Vec<Field>,
    pub grid:       Rc<RefCell<Grid>>,
}

impl Row {
    pub fn new(grid: &Rc<RefCell<Grid>>,
               direction: Direction,
               index: usize,
               run_lengths: &Vec<usize>) -> Self
    {
        let length = match direction {
            Horizontal => grid.borrow().width(),
            Vertical   => grid.borrow().height(),
        };
        let runs = run_lengths.iter()
                              .enumerate()
                              .map(|(i, &len)| Run::new(grid, direction, i, index, len))
                              .collect::<Vec<_>>();
        Row {
            direction: direction,
            index:     index,
            length:    length,
            runs:      runs,
            fields:    Vec::<Field>::new(),
            grid:      Rc::clone(grid),
        }
    }
    pub fn range(&self) -> Range<usize> {
        0..self.length
    }

    pub fn get_square(&self, index: usize) -> Ref<Square> {
        let grid = self.grid.borrow();
        match self.direction {
            Horizontal => Ref::map(grid, |g| g.get_square(index, self.index) ),
            Vertical   => Ref::map(grid, |g| g.get_square(self.index, index) ),
        }
    }
    pub fn get_square_mut(&self, index: usize) -> RefMut<Square> {
        let grid = self.grid.borrow_mut();
        match self.direction {
            Horizontal => RefMut::map(grid, |g| g.get_square_mut(index, self.index) ),
            Vertical   => RefMut::map(grid, |g| g.get_square_mut(self.index, index) ),
        }
    }

    pub fn make_field(&self, offset: usize, length: usize) -> Field {
        Field::new(self.direction, offset, length, self.index, &self.grid)
    }

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
            while x < self.length && pred(self.get_square(x)) { // TODO: first iteration runs pred twice :(
                x += 1;
            }
            let range_end = x;
            result.push(range_start..range_end);
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
    pub fn fill_overlap(&self)
    {
        for run in &self.runs
        {
            let max_start = run.max_start.unwrap();
            let min_start = run.min_start.unwrap();
            let diff = max_start - min_start;

            if diff < run.length {
                // found overlap
                let overlap_start = max_start;
                let overlap_len   = run.length - diff;
                for i in 0..overlap_len {
                    let mut square: RefMut<Square> = self.get_square_mut(overlap_start+i);
                    square.set_status(FilledIn).expect("Failed to set square state");
                    square.assign_run(run).expect("Failed to set square run");
                }
                if diff == 0 {
                    // found exact match; cross out squares to the left and right
                    // to isolate this run into its own field
                    if overlap_start > 0 {
                        self.get_square_mut(overlap_start-1).set_status(CrossedOut).expect("");
                    }
                    if overlap_start + overlap_len < self.length-1 {
                        self.get_square_mut(overlap_start+overlap_len).set_status(CrossedOut).expect("");
                    }
                }
            }
        }
    }
    pub fn mark_completed_runs(&mut self){
        // scan each field; if all squares in the field are assigned the same run,
        // (and the field has the same length as the run), then this run is complete.
        /*for field in &self.fields {
            if field.range()
                    .map(|i| self.get_square(i).get_run_index)
        }*/
    }

}

#[derive(Debug)]
pub struct Run {
    pub direction: Direction,
    pub length: usize,
    pub index: usize,
    pub row_index: usize,
    pub grid: Rc<RefCell<Grid>>,
    //
    pub min_start: Option<usize>,
    pub max_start: Option<usize>,
    pub completed: bool,
}

impl Run {
    pub fn new(grid: &Rc<RefCell<Grid>>,
               direction: Direction,
               index: usize,
               row_index: usize,
               length: usize) -> Self
    {
        Run {
            direction,
            length,
            index,
            row_index,
            grid: Rc::clone(grid),
            min_start: None,
            max_start: None,
            completed: false,
        }
    }
}
impl fmt::Display for Run {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.length)
    }
}

#[derive(Debug)]
pub struct Field {
    pub direction: Direction,
    pub offset: usize,
    pub length: usize,
    pub row_index: usize,
    pub grid: Rc<RefCell<Grid>>,
}

impl Field {
    pub fn new(direction: Direction,
               offset: usize,
               length: usize,
               row_index: usize,
               grid: &Rc<RefCell<Grid>>) -> Self
    {
        Field {
            offset,
            length,
            direction,
            row_index,
            grid: Rc::clone(grid),
        }
    }
    pub fn range(&self) -> Range<usize> {
        self.offset..self.offset+self.length
    }
    pub fn run_fits(&self, run: &Run) -> bool {
        self.run_lfits_at(run, 0)
    }
    pub fn run_lfits_at(&self, run: &Run, l_shift: usize) -> bool {
        // does the given run fit in this field, starting at the given relative shift within the field?
        // e.g.:
        //       0 1 2 3 4 5 6 7
        //     [ . . . . . . . . ]
        //                 |
        //                 shift
        // => a run of length 3 would fit at shift=5 in a field of length 8
        l_shift + run.length <= self.length
    }
    pub fn run_rfits_at(&self, run: &Run, r_shift: usize) -> bool {
        // same as run_lfits_at, but for a run ending at the given relative shift position from the right
        // within the field (an r_shift of 0 signifies ending exactly at the right boundary)

        // actually identical to the lfits case except flipped 180 degrees,
        // we're just "filling up" the field starting from the right side and going to the left now
        r_shift + run.length <= self.length
    }
}
