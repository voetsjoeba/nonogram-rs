// vim: set ai et ts=4 sw=4 sts=4:
//use std::iter::Iterator;
mod solver;

use std::fmt;
use std::ops::Range;
use std::convert::{TryInto, TryFrom};
use std::cmp::{min, max};
use std::rc::{Rc, Weak};
use std::cell::{Ref, RefMut, RefCell};
use std::collections::HashSet;
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
