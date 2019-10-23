// vim: set ai et ts=4 sts=4:
use std::fmt;
use std::convert::From;
use super::util::{Direction, Direction::*};
use super::row::Run;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum SquareStatus {
    FilledIn,
    CrossedOut,
    Unknown,
}
impl fmt::Display for SquareStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            SquareStatus::FilledIn => "FilledIn",
            SquareStatus::CrossedOut => "CrossedOut",
            SquareStatus::Unknown => "Unknown",
        })
    }
}

#[derive(PartialEq, Debug)]
pub struct StatusChange {
    pub row: usize,
    pub col: usize,
    pub old: SquareStatus,
    pub new: SquareStatus,
}
impl StatusChange {
    fn new(sq: &Square, old: SquareStatus, new: SquareStatus) -> Self {
        Self { row: sq.row, col: sq.col, old, new }
    }
}
impl fmt::Display for StatusChange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Change: in square (col={}, row={}), status was changed from {} to {}",
            self.col,
            self.row,
            self.old,
            self.new)
    }
}

#[derive(PartialEq, Debug)]
pub struct RunChange {
    pub row: usize,
    pub col: usize,
    pub direction: Direction,
    pub old: Option<usize>,
    pub new: usize,
}
impl RunChange {
    fn new(sq: &Square, direction: Direction, old: Option<usize>, new: usize) -> Self {
        Self { row: sq.row, col: sq.col, direction, old, new }
    }
}
impl fmt::Display for RunChange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Change: in square (col={}, row={}), {} run index was changed from {} to {}",
            self.col,
            self.row,
            self.direction,
            match self.old {
                None    => "None".to_string(),
                Some(x) => x.to_string(),
            },
            self.new)
    }
}

pub enum Change {
    Status(StatusChange),
    Run(RunChange),
}
impl From<StatusChange> for Change {
    fn from(other: StatusChange) -> Self {
        Change::Status(other)
    }
}
impl From<RunChange> for Change {
    fn from(other: RunChange) -> Self {
        Change::Run(other)
    }
}
impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Change::Status(x) => x.to_string(),
            Change::Run(x)    => x.to_string(),
        })
    }
}
pub type Changes = Vec<Change>;

// ------------------------------------------------

#[derive(PartialEq, Debug)]
pub enum StatusError {
    ChangeRejected(StatusChange, String),  // new status conflicts with existing (non-unknown) status
}
impl fmt::Display for StatusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StatusError: {}", match self {
            StatusError::ChangeRejected(change, msg) =>
                format!("In (col={}, row={}), attempt to change status from {} to {} was rejected: {}",
                    change.col, change.row, change.old, change.new, msg),
        })
    }
}

#[derive(PartialEq, Debug)]
pub enum RunError {
    ChangeRejected(RunChange, String) // new run assignment conflicts with existing one
}
impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RunError: {}", match self {
            RunError::ChangeRejected(change, msg) =>
                format!("In (col={}, row={}), attempt to change {} run index from {} to {} was rejected: {}",
                    change.col, change.row, change.direction, match change.old {
                        Some(x) => x.to_string(),
                        None    => "None".to_string(),
                    }, change.new, msg),
        })
    }
}

pub type StatusResult = Result<Option<StatusChange>, StatusError>; // if it worked: the change, if any; if it didn't, the change that was rejected
pub type RunResult    = Result<Option<RunChange>, RunError>; // ditto

pub enum Error {
    Status(StatusError),
    Run(RunError),
}
impl From<StatusError> for Error {
    fn from(other: StatusError) -> Self {
        Error::Status(other)
    }
}
impl From<RunError> for Error {
    fn from(other: RunError) -> Self {
        Error::Run(other)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Error::Status(x) => x.to_string(),
            Error::Run(x)    => x.to_string(),
        })
    }
}

// ------------------------------------------------


// ------------------------------------------------

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

    pub fn set_status(&mut self, new_status: SquareStatus) -> StatusResult
    {
        let cand_change = StatusChange::new(&self, self.status, new_status);
        // if this square's status is already known, it can't be changed anymore,
        // otherwise that's a conflict
        if self.status != SquareStatus::Unknown {
            if self.status != new_status {
                return Err(StatusError::ChangeRejected(cand_change, "conflicting information".to_string()));
            }
        }

        if self.status != new_status {
            self.status = new_status;
            return Ok(Some(cand_change));
        }
        return Ok(None);
    }

    pub fn get_run_index(&self, direction: Direction) -> Option<usize> {
        match direction {
            Horizontal => self.hrun_index,
            Vertical   => self.vrun_index,
        }
    }
    pub fn set_run_index(&mut self, direction: Direction, new_index: usize)
        -> RunResult
    {
        match direction {
            Horizontal => {
                let cand_change = RunChange::new(&self, direction, self.hrun_index, new_index);
                if let Some(x) = self.hrun_index {
                    if x != new_index {
                        return Err(RunError::ChangeRejected(cand_change, "conflicting information".to_string()));
                    }
                }
                if self.hrun_index == None || self.hrun_index != Some(new_index) {
                    self.hrun_index = Some(new_index);
                    return Ok(Some(cand_change));
                } else {
                    return Ok(None);
                }
            },
            Vertical   => {
                let cand_change = RunChange::new(&self, direction, self.vrun_index, new_index);
                if let Some(x) = self.vrun_index {
                    if x != new_index {
                        return Err(RunError::ChangeRejected(cand_change, "conflicting information".to_string()));
                    }
                }
                if self.vrun_index == None || self.vrun_index != Some(new_index) {
                    self.vrun_index = Some(new_index);
                    return Ok(Some(cand_change));
                } else {
                    return Ok(None);
                }
            },
        }
    }
    pub fn assign_run(&mut self, run: &Run) -> RunResult {
        self.set_run_index(run.direction, run.index)
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self.status {
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

