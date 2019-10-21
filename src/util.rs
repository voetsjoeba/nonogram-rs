// vim: set ai et ts=4 sw=4 sts=4:
use std::fmt;

pub fn ralign(s: &str, width: usize) -> String {
    if s.len() >= width {
        return String::from(s);
    }
    format!("{}{}", " ".repeat(width-s.len()), s)
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Direction {
    Horizontal,
    Vertical,
}
impl fmt::Display for Direction {
    fn fmt(&self,
           f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "{}", match self {
            Direction::Horizontal => "Horizontal",
            Direction::Vertical   => "Vertical",
        })
    }
}
