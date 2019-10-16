// vim: set ai et ts=4 sts=4 sw=4:
use super::util::Direction;

#[derive(Debug)]
pub struct Run {
    pub direction: Direction,
    pub length: u32,
    min_start: i32,
    max_start: i32,
    completed: bool,
}

impl Run {
    pub fn new(direction: Direction, length: u32) -> Self {
        Run {
            direction,
            length,
            min_start: -1,
            max_start: -1,
            completed: false,
        }
    }
}

