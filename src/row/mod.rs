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
use ansi_term::{Colour, Style, ANSIString};

use super::util::{Direction, Direction::*};
use super::grid::{Grid, Square, SquareStatus::{CrossedOut, FilledIn}, Change, Changes, Error};

pub trait DirectionalSequence
{
    fn get_row_index(&self) -> usize;
    fn get_direction(&self) -> Direction;
    fn get_grid(&self) -> &Rc<RefCell<Grid>>;

    fn square_index(&self, at: usize) -> (usize, usize) {
        match self.get_direction() {
            Horizontal => (at, self.get_row_index()),
            Vertical   => (self.get_row_index(), at),
        }
    }
    fn get_square(&self, index: usize) -> Ref<Square> {
        let grid = self.get_grid().borrow();
        let (x,y) = self.square_index(index);
        Ref::map(grid, |g| g.get_square(x, y))
    }
    fn get_square_mut(&self, index: usize) -> RefMut<Square> {
        let grid = self.get_grid().borrow_mut();
        let (x,y) = self.square_index(index);
        RefMut::map(grid, |g| g.get_square_mut(x, y))
    }
}

#[derive(Debug)]
pub struct Row {
    pub direction:  Direction,
    pub index:      usize,
    pub length:     usize,
    pub runs:       Vec<Run>,
    pub fields:     Vec<Field>,
    pub grid:       Rc<RefCell<Grid>>,
    pub completed:  bool,
}

impl Row {
    pub fn new(grid: &Rc<RefCell<Grid>>,
               direction: Direction,
               row_index: usize,
               run_lengths: &Vec<usize>) -> Self
    {
        let row_length = match direction {
            Horizontal => grid.borrow().width(),
            Vertical   => grid.borrow().height(),
        };
        let runs = run_lengths.iter()
                              .enumerate()
                              .map(|(i, &len)| Run::new(grid, direction, i, row_index, row_length, len))
                              .collect::<Vec<_>>();
        Row {
            direction: direction,
            index:     row_index,
            length:    row_length,
            runs:      runs,
            fields:    Vec::<Field>::new(),
            grid:      Rc::clone(grid),
            completed: false,
        }
    }

    pub fn make_field(&self, offset: usize, length: usize) -> Field {
        Field::new(self.direction, offset, length, self.index, &self.grid)
    }
    pub fn is_completed(&self) -> bool {
        self.completed
    }

}
impl DirectionalSequence for Row {
    fn get_row_index(&self) -> usize { self.index }
    fn get_direction(&self) -> Direction { self.direction }
    fn get_grid(&self)      -> &Rc<RefCell<Grid>> { &self.grid }
}

#[derive(Debug)]
pub struct Run {
    pub direction: Direction,
    pub length: usize,
    pub index: usize,
    pub row_index: usize,
    pub row_length: usize,
    pub grid: Rc<RefCell<Grid>>,
    //
    pub min_start: Option<usize>,
    pub max_start: Option<usize>,
    completed: bool,
}

impl Run {
    pub fn new(grid: &Rc<RefCell<Grid>>,
               direction: Direction,
               index: usize,
               row_index: usize,
               row_length: usize,
               length: usize) -> Self
    {
        Run {
            direction,
            length,
            index,
            row_index,
            row_length,
            grid: Rc::clone(grid),
            min_start: None,
            max_start: None,
            completed: false,
        }
    }
}
impl Run {
    pub fn complete(&mut self, at: usize) -> Result<Changes, Error> {
        // found position for this run; cross out squares to the left and right of this run
        let mut changes = Vec::<Change>::new();
        if at > 0 {
            if let Some(change) = self.get_square_mut(at-1).set_status(CrossedOut)? {
                changes.push(Change::from(change));
            }
        }
        if at + self.length < self.row_length {
            if let Some(change) = self.get_square_mut(at + self.length).set_status(CrossedOut)? {
                changes.push(Change::from(change));
            }
        }
        self.completed = true;
        Ok(changes)
    }
    pub fn is_completed(&self) -> bool {
        self.completed
    }
    pub fn to_colored_string(&self) -> ANSIString {
        let style = match self.completed {
            true  => Style::new().fg(Colour::Fixed(241)),
            false => Style::default(),
        };
        style.paint(self.to_string())
    }
    pub fn is_possible_start_position(&self, start: usize) -> bool {
        // returns true if this run has its min/max_start bounds set, and the given
        // starting position falls within that range
        self.min_start.is_some() && self.max_start.is_some() &&
        self.min_start.unwrap() <= start && start <= self.max_start.unwrap()
    }
}
impl DirectionalSequence for Run {
    fn get_row_index(&self) -> usize { self.row_index }
    fn get_direction(&self) -> Direction { self.direction }
    fn get_grid(&self)      -> &Rc<RefCell<Grid>> { &self.grid }
}
impl fmt::Display for Run {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.length.to_string())
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
