// vim: set ai et ts=4 sts=4:
use std::fmt;
use std::convert::{From, TryFrom};
use std::rc::{Rc};
use std::cell::{RefCell};
use super::util::{Direction, Direction::*};
use super::row::Run;

pub trait HasGridLocation {
    fn get_row(&self) -> usize;
    fn get_col(&self) -> usize;
    fn fmt_location(&self) -> String {
        format!("(col={:-2}, row={:-2})", self.get_col(), self.get_row())
    }
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum SquareStatus {
    FilledIn,
    CrossedOut,
    Unknown,
}
impl fmt::Display for SquareStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            SquareStatus::FilledIn   => "FilledIn",
            SquareStatus::CrossedOut => "CrossedOut",
            SquareStatus::Unknown    => "Unknown",
        })
    }
}
impl TryFrom<&str> for SquareStatus {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "FilledIn"   => Ok(SquareStatus::FilledIn),
            "CrossedOut" => Ok(SquareStatus::CrossedOut),
            "Unknown"    => Ok(SquareStatus::Unknown),
            _            => Err("Not a valid SquareStatus value")
        }
    }
}

// ------------------------------------------------

#[derive(PartialEq, Debug, Clone)]
pub struct StatusChange {
    pub row: usize,
    pub col: usize,
    pub old: SquareStatus,
    pub new: SquareStatus,
}
impl StatusChange {
    pub fn new(row: usize, col: usize, old: SquareStatus, new: SquareStatus) -> Self {
        Self { row, col, old, new }
    }
}
impl HasGridLocation for StatusChange {
    fn get_row(&self) -> usize { self.row }
    fn get_col(&self) -> usize { self.col }
}
impl fmt::Display for StatusChange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Change: in square {}, status was changed from {} to {}",
            self.fmt_location(),
            self.old,
            self.new)
    }
}

// ------------------------------------------------

#[derive(PartialEq, Debug, Clone)]
pub struct RunChange {
    pub row: usize,
    pub col: usize,
    pub direction: Direction,
    pub old: Option<usize>,
    pub new: usize,
}
impl RunChange {
    pub fn new(row: usize, col: usize, direction: Direction, old: Option<usize>, new: usize) -> Self {
        Self { row, col, direction, old, new }
    }
}
impl HasGridLocation for RunChange {
    fn get_row(&self) -> usize { self.row }
    fn get_col(&self) -> usize { self.col }
}
impl fmt::Display for RunChange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Change: in square {}, {} run index was changed from {} to {}",
            self.fmt_location(),
            self.direction,
            match self.old {
                None    => "None".to_string(),
                Some(x) => x.to_string(),
            },
            self.new)
    }
}

// ------------------------------------------------

#[derive(Debug, Clone)]
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
impl HasGridLocation for Change {
    fn get_row(&self) -> usize {
        match self {
            Change::Status(x) => x.get_row(),
            Change::Run(x)    => x.get_row(),
        }
    }
    fn get_col(&self) -> usize {
        match self {
            Change::Status(x) => x.get_col(),
            Change::Run(x)    => x.get_col(),
        }
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
                format!("In {}, attempt to change status from {} to {} was rejected: {}",
                    change.fmt_location(), change.old, change.new, msg),
        })
    }
}

#[derive(PartialEq, Debug)]
pub enum RunError {
    ChangeRejected(RunChange, String), // new run assignment conflicts with existing one
    NotFilledIn(RunChange),            // can't assign a run to a square that's not filled in
}
impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RunError: {}", match self {
            RunError::ChangeRejected(change, msg) =>
                format!("In {}, attempt to change {} run index from {} to {} was rejected: {}",
                    change.fmt_location(), change.direction, match change.old {
                        Some(x) => x.to_string(),
                        None    => "None".to_string(),
                    }, change.new, msg),
            RunError::NotFilledIn(change) =>
                format!("In {}, cannot set {} run index to {}: square is not filled in",
                    change.fmt_location(), change.direction, change.new),
        })
    }
}

pub type StatusResult = Result<Option<StatusChange>, StatusError>; // if it worked: the change, if any; if it didn't, the change that was rejected
pub type RunResult    = Result<Option<RunChange>, RunError>; // ditto

#[derive(Debug)]
pub enum Error {
    Status(StatusError),
    Run(RunError),
    Logic(String),
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
            Error::Logic(s)  => s.to_string(),
        })
    }
}

// ------------------------------------------------

#[derive(Debug, Clone)]
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
        let cand_change = StatusChange::new(self.row, self.col, self.status, new_status);
        self.apply_status_change(cand_change)
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
        let cand_change = RunChange::new(self.row, self.col, direction, self.get_run_index(direction), new_index);
        self.apply_run_change(cand_change)
    }
    pub fn assign_run(&mut self, run: &Run) -> RunResult {
        self.set_run_index(run.direction, run.index)
    }
    pub fn has_run_assigned(&self, run: &Run) -> bool {
        self.get_run_index(run.direction) == Some(run.index)
    }
    pub fn apply_status_change(&mut self, cand_change: StatusChange)
        -> StatusResult
    {
        assert!(cand_change.row == self.row);
        assert!(cand_change.col == self.col);

        // if this square's status is already known, it can't be changed anymore,
        // that would be a conflict
        if self.status != SquareStatus::Unknown {
            if self.status != cand_change.new {
                return Err(StatusError::ChangeRejected(cand_change, "conflicting information".to_string()));
            }
        }
        if self.status != cand_change.new {
            self.status = cand_change.new;
            return Ok(Some(cand_change));
        }
        return Ok(None);
    }
    pub fn apply_run_change(&mut self, cand_change: RunChange)
        -> RunResult
    {
        assert!(cand_change.row == self.row);
        assert!(cand_change.col == self.col);

        let field = match cand_change.direction {
            Horizontal => &mut self.hrun_index,
            Vertical   => &mut self.vrun_index,
        };
        if self.status != SquareStatus::FilledIn {
            return Err(RunError::NotFilledIn(cand_change))
        }
        if let Some(x) = *field {
            if x != cand_change.new {
                return Err(RunError::ChangeRejected(cand_change, "conflicting information".to_string()));
            }
        }
        if *field == None || *field != Some(cand_change.new) {
            *field = Some(cand_change.new);
            return Ok(Some(cand_change));
        } else {
            return Ok(None);
        }
    }
    pub fn apply_change(&mut self, cand_change: Change)
        -> Result<Option<Change>, Error>
    {
        match cand_change {
            Change::Status(status_change)
                => match self.apply_status_change(status_change) {
                       Ok(None)          => Ok(None),
                       Ok(Some(x))       => Ok(Some(Change::from(x))),
                       Err(status_error) => Err(Error::from(status_error)),
                   }
            Change::Run(run_change)
                => match self.apply_run_change(run_change) {
                       Ok(None)          => Ok(None),
                       Ok(Some(x))       => Ok(Some(Change::from(x))),
                       Err(run_error)    => Err(Error::from(run_error)),
                   }
        }
    }

    pub fn fmt_visual(&self) -> &str {
        match self.status {
            SquareStatus::CrossedOut => " ",
            SquareStatus::FilledIn   => "\u{25A0}",
            SquareStatus::Unknown    => ".",
        }
    }
}
impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.fmt_visual())
    }
}
impl HasGridLocation for Square {
    fn get_row(&self) -> usize { self.row }
    fn get_col(&self) -> usize { self.col }
}

// ------------------------------------------------

#[derive(Clone)]
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

// ------------------------------------------------

pub trait CloneGridAware {
    // clones a struct that carries an Rc<RefCell<Grid>>
    fn clone_with_grid(&self, grid: &Rc<RefCell<Grid>>) -> Self;
}

