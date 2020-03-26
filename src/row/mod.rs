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
use super::grid::{Grid, Square, SquareStatus::{CrossedOut, FilledIn}, Change, Changes, Error, CloneGridAware};

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
            grid:      Rc::clone(grid),
            completed: false,
        }
    }
    fn _ranges_of_squares<P>(&self, pred: P) -> Vec<Range<usize>>
        where P: Fn(Ref<Square>, usize) -> bool
    {
        // given a predicate on a square, returns a set of mutually exclusive ranges within this row 
        // for which the predicate holds for all squares in that range.
        let mut result = Vec::<Range<usize>>::new();
        let mut x: usize = 0;
        while x < self.length {
            // skip past squares for which the predicate does not hold
            while x < self.length && !pred(self.get_square(x), x) {
                x += 1;
            }
            if x >= self.length { break; }

            // skip past squares for which the predicate does hold
            let range_start = x;
            x += 1; // we already tested the predicate on x at the end of the previous loop
            while x < self.length && pred(self.get_square(x), x) {
                x += 1;
            }
            let range_end = x;
            result.push(range_start..range_end);

            x += 1;
        }
        result
    }
    fn _ranges_of_runs<P>(&self, pred: P) -> Vec<Range<usize>>
        where P: Fn(&Run) -> bool
    {
        // given a predicate on a run, returns a set of mutually exclusive ranges
        // of contiguous run indices for which the predicate holds.
        let mut result = Vec::<Range<usize>>::new();
        let mut x: usize = 0;
        while x < self.runs.len() {
            // skip past squares for which the predicate does not hold
            while x < self.runs.len() && !pred(&self.runs[x]) {
                x += 1;
            }
            if x >= self.runs.len() { break; }

            // skip past runs for which the predicate does hold
            let range_start = x;
            x += 1; // we already tested the predicate on x at the end of the previous loop
            while x < self.runs.len() && pred(&self.runs[x]) {
                x += 1;
            }
            let range_end = x;
            result.push(range_start..range_end);

            x += 1;
        }
        result
    }

    pub fn get_fields(&self) -> Vec<Range<usize>> {
        // returns the set of ranges in this row of contiguous squares that are not crossed out
        self._ranges_of_squares(|sq, _| sq.get_status() != CrossedOut)
    }

    pub fn is_completed(&self) -> bool {
        self.completed
    }
    pub fn is_trivially_empty(&self) -> bool {
        self.runs.is_empty() || self.runs.iter().all(|r| r.length == 0)
    }
    pub fn possible_runs_for_sequence(&self, seq: &Range<usize>) -> Vec<usize>
    {
        // answers the question: if a sequence of filled in squares would be placed
        // at the given position, which runs could that sequence (in its entirety) belong to?

        // the conditions for a run to be eligible to contain the sequence are:
        //  - the run must be at least as long as the sequence.
        //  - the run must contain a possible placement that contains ALL squares in the sequence
        //    (or equivalently: it must contain BOTH the start and end square in the sequence)
        self.runs.iter()
                 .filter(|run| run.length >= seq.len()
                               && run.possible_placements.iter()
                                                         .any(|range| range.contains(&seq.start)
                                                                      && range.contains(&(seq.end-1))))
                 .map(|run| run.index)
                 .collect()
    }
    pub fn possible_runs_for_square(&self, position: usize) -> Vec<usize> {
        self.possible_runs_for_sequence(&(position..(position+1)))
    }
}
impl DirectionalSequence for Row {
    fn get_row_index(&self) -> usize { self.index }
    fn get_direction(&self) -> Direction { self.direction }
    fn get_grid(&self)      -> &Rc<RefCell<Grid>> { &self.grid }
}

impl CloneGridAware for Row {
    fn clone_with_grid(&self, grid: &Rc<RefCell<Grid>>) -> Self {
        Row {
            direction:    self.direction.clone(),
            index:        self.index.clone(),
            length:       self.length.clone(),
            completed:    self.completed.clone(),
            runs:         self.runs.iter().map(|run| run.clone_with_grid(grid)).collect(),
            grid:         Rc::clone(grid),
        }
    }
}

// -------------------------------------------------------------

#[derive(Debug)]
pub struct Run {
    pub direction: Direction,
    pub length: usize,
    pub index: usize,
    pub row_index: usize,
    pub row_length: usize,
    pub grid: Rc<RefCell<Grid>>,
    pub possible_placements: Vec<Range<usize>>,
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
            possible_placements: Vec::<Range<usize>>::new(),
            completed: false,
        }
    }
}
impl Run {
    pub fn complete(&mut self, start_at: usize) -> Result<Changes, Error> {
        // found final position for this run; cross out squares to the left and right,
        // and set the final position as its only possible placement.
        let mut changes = Vec::<Change>::new();
        changes.extend(self.delineate_at(start_at)?);
        self.possible_placements = vec![start_at..start_at+self.length];
        self.completed = true;
        Ok(changes)
    }
    pub fn delineate_at(&mut self, start_at: usize) -> Result<Changes, Error> {
        // assuming that this run will be placed at the given starting position,
        // cross out squares directly in front and behind of it
        let mut changes = Vec::<Change>::new();
        if start_at > 0 {
            if let Some(change) = self.get_square_mut(start_at-1).set_status(CrossedOut)? {
                changes.push(Change::from(change));
            }
        }
        if start_at + self.length < self.row_length {
            if let Some(change) = self.get_square_mut(start_at + self.length).set_status(CrossedOut)? {
                changes.push(Change::from(change));
            }
        }
        Ok(changes)
    }
    pub fn is_completed(&self) -> bool {
        self.completed
    }
    pub fn completed_placement(&self) -> Range<usize> {
        assert!(self.is_completed());
        assert!(self.possible_placements.len() == 1);
        self.possible_placements[0].clone()
    }
    pub fn to_colored_string(&self) -> ANSIString {
        let style = match self.completed {
            true  => Style::new().fg(Colour::Fixed(241)),
            false => Style::default(),
        };
        style.paint(self.to_string())
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

impl CloneGridAware for Run {
    fn clone_with_grid(&self, grid: &Rc<RefCell<Grid>>) -> Self {
        Run {
            direction:             self.direction.clone(),
            length:                self.length.clone(),
            index:                 self.index.clone(),
            row_index:             self.row_index.clone(),
            row_length:            self.row_length.clone(),
            possible_placements:   self.possible_placements.clone(),
            completed:             self.completed.clone(),
            grid:                  Rc::clone(grid),
        }
    }
}

