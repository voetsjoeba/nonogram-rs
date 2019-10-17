// vim: set ai et ts=4 sw=4 sts=4:
use std::rc::{Rc, Weak};
use std::cell::{Ref, RefCell};
use super::util::{Direction, Direction::*};
use super::puzzle::Puzzle;
use super::grid::{Grid, Square};

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
               run_lengths: &Vec<usize>)
        -> Self
    {
        let length = match direction {
            Horizontal => grid.borrow().width(),
            Vertical   => grid.borrow().height(),
        };
        let runs = run_lengths.iter()
                              .map(|&len| Run::new(grid, direction, index, len))
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

    pub fn get_square(&self, index: usize) -> Ref<Square> {
        let grid = self.grid.borrow();
        match self.direction {
            Horizontal => Ref::map(grid, |g| g.get_square(index, self.index) ),
            Vertical   => Ref::map(grid, |g| g.get_square(self.index, index) ),
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
    pub row_index: usize,
    pub grid: Rc<RefCell<Grid>>,
    //
    pub min_start: usize,
    pub max_start: usize,
    pub completed: bool,
}

impl Run {
    pub fn new(grid: &Rc<RefCell<Grid>>,
               direction: Direction,
               row_index: usize,
               length: usize)
        -> Self
    {
        Run {
            direction,
            row_index,
            length,
            grid: Rc::clone(grid),
            min_start: 0,
            max_start: 0,
            completed: false,
        }
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
               grid: &Rc<RefCell<Grid>>)
        -> Self
    {
        Field {
            offset,
            length,
            direction,
            row_index,
            grid: Rc::clone(grid),
        }
    }
}
