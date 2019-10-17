// vim: set ai et ts=4 sts=4:
use std::fmt;

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

type StatusResult = Result<(), StatusError>;

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
}

impl fmt::Debug for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Grid(w={}, h={})", self.width(), self.height())
    }
}

