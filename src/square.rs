// vim: set ai et ts=4 sts=4:
use std::fmt;
use std::result;

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum SquareStatus {
    FilledIn,
    CrossedOut,
    Unknown,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum StatusError {
    WasAlreadySet, // status was already filled in or crossed out, cannot be reverted to unknown
    Conflicts,     // new status conflicts with existing (non-unknown) status
}

type StatusResult = result::Result<(), StatusError>;

#[derive(Debug)]
pub struct Square {
    row: usize,
    col: usize,
    status: SquareStatus,
}

impl Square {
    pub fn new(x: usize, y: usize) -> Square {
        Square {
            row: y,
            col: x,
            status: SquareStatus::Unknown,
        }
    }

    pub fn get_row(&self) -> usize { self.row }
    pub fn get_col(&self) -> usize { self.col }
    pub fn get_status(&self) -> SquareStatus { self.status }

    pub fn set_status(&mut self, new_status: SquareStatus) -> StatusResult {
        if self.status != SquareStatus::Unknown {
            if new_status == SquareStatus::Unknown { return Err(StatusError::WasAlreadySet) }
            if self.status != new_status           { return Err(StatusError::Conflicts);    }
        }

        self.status = new_status;
        return Ok(());
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self.status {
            SquareStatus::CrossedOut => "x",
            SquareStatus::FilledIn   => "\u{25A0}", // filled in black square
            SquareStatus::Unknown    => "\u{26AC}", // medium circle, not filled in
        })
    }
}

