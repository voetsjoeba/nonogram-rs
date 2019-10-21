// vim: set ai et ts=4 sts=4:
use std::fmt;
use super::util::{Direction, Direction::*};
use super::row::Run;

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum SquareStatus {
    FilledIn,
    CrossedOut,
    Unknown,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum StatusError {
    WasAlreadySet(SquareStatus),  // status was already filled in or crossed out, cannot be reverted to unknown
    Conflicts(SquareStatus),      // new status conflicts with existing (non-unknown) status
}
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum RunAssignmentError {
    Conflicts(Direction, usize),
}

type StatusResult = Result<(), StatusError>;
type RunAssignmentResult = Result<(), RunAssignmentError>;

#[derive(Debug)]
pub struct Square {
    row: usize,
    col: usize,
    status: SquareStatus,
    hrun_index: Option<usize>, // index of run in horizontal row that this square belongs to
    vrun_index: Option<usize>, // ...             vertical   ...
}

impl Square {
    pub fn new(x: usize, y: usize) -> Square {
        Square {
            row: y,
            col: x,
            status: SquareStatus::Unknown,
            hrun_index: None,
            vrun_index: None,
        }
    }

    pub fn get_row(&self) -> usize { self.row }
    pub fn get_col(&self) -> usize { self.col }
    pub fn get_status(&self) -> SquareStatus { self.status }

    pub fn set_status(&mut self, new_status: SquareStatus) -> StatusResult {
        if self.status != SquareStatus::Unknown {
            if new_status == SquareStatus::Unknown { return Err(StatusError::WasAlreadySet(self.status)) }
            if self.status != new_status           { return Err(StatusError::Conflicts(self.status));    }
        }

        self.status = new_status;
        return Ok(());
    }

    pub fn get_run_index(&self, direction: Direction) -> Option<usize> {
        match direction {
            Horizontal => self.hrun_index,
            Vertical   => self.vrun_index,
        }
    }
    pub fn set_run_index(&mut self, direction: Direction, new_index: usize)
        -> RunAssignmentResult
    {
        match direction {
            Horizontal => {
                if let Some(x) = self.hrun_index {
                    if x != new_index { return Err(RunAssignmentError::Conflicts(direction, x)); }
                }
                self.hrun_index = Some(new_index);
                return Ok(());
            },
            Vertical   => {
                if let Some(x) = self.vrun_index {
                    if x != new_index { return Err(RunAssignmentError::Conflicts(direction, x)); }
                }
                self.vrun_index = Some(new_index);
                return Ok(());
            },
        }
    }
    pub fn assign_run(&mut self, run: &Run) -> RunAssignmentResult {
        self.set_run_index(run.direction, run.index)
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self.status {
            //SquareStatus::CrossedOut => "x",
            //SquareStatus::FilledIn   => "\u{25A0}", // filled in black square
            //SquareStatus::Unknown    => "\u{26AC}", // medium circle, not filled in

            SquareStatus::CrossedOut => " ",
            SquareStatus::FilledIn   => "\u{25A0}",
            SquareStatus::Unknown    => ".",
        })
    }
}

pub struct Grid {
    pub squares: Vec<Vec<Square>>,
}
impl Grid {
    pub fn new(width: usize, height: usize)
        -> Self
    {
        Grid {
            squares: (0..height).map(|y| (0..width).map(|x| Square::new(x, y))
                                                   .collect::<Vec<_>>())
                                .collect(),
        }
    }

    pub fn width(&self) -> usize { self.squares[0].len() }
    pub fn height(&self) -> usize { self.squares.len() }
    pub fn get_square(&self, x: usize, y: usize) -> &Square {
        &self.squares[y][x]
    }
    pub fn get_square_mut(&mut self, x: usize, y: usize) -> &mut Square {
        &mut self.squares[y][x]
    }
}

impl fmt::Debug for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Grid(w={}, h={})", self.width(), self.height())
    }
}

