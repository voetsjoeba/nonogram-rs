// vim: set ai et ts=4 sw=4 sts=4:
use super::util::Direction;
use super::run::Run;
use super::field::Field;

#[derive(Debug)]
pub struct Row {
    pub direction:  Direction,
    pub index:      u32,
    pub runs:       Vec<Run>,
    pub fields:     Vec<Field>,
}

impl Row {
    pub fn new(direction: Direction, index: u32, runs: Vec<Run>) -> Self {
        Row {
            direction,
            index,
            runs,
            fields: Vec::<Field>::new()
        }
    }
}

