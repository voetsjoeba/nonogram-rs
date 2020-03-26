// vim: set ai et ts=4 sw=4 sts=4:
use std::fmt;
use std::io;
use std::convert::TryFrom;
use std::os::unix::io::AsRawFd;
use std::rc::Rc;
use std::cell::RefCell;
use ansi_term::ANSIString;

pub fn maybe_color(s: &ANSIString, emit_color: bool) -> String {
    match emit_color {
        true  => s.to_string(),
        false => (**s).to_string(), // deref once to get ANSIString, once more to get underlying str
    }
}
pub fn ralign(s: &str, width: usize) -> String {
    if s.len() >= width {
        return String::from(s);
    }
    format!("{}{}", " ".repeat(width-s.len()), s)
}
pub fn lalign_colored(s: &ANSIString, width: usize, emit_color: bool)
    -> String
{
    let visual_len = s.len(); // ANSIString.len() returns length WITHOUT escape sequences
    if visual_len >= width {
        return maybe_color(s, emit_color);
    }
    format!("{}{}", maybe_color(s, emit_color), " ".repeat(width-visual_len))
}
pub fn ralign_joined_coloreds(strs: &Vec<ANSIString>, width: usize, emit_color: bool)
    -> String
{
    let mut visual_len: usize = strs.iter().map(|ansi_str| ansi_str.len()).sum(); // ANSIString.len() returns length WITHOUT escape sequences
    visual_len += strs.len()-1; // count the spaces that .join(" ") will add

    let joined_colored = strs.iter()
                             .map(|astr| maybe_color(astr, emit_color))
                             .collect::<Vec<_>>()
                             .join(" ");
    if visual_len >= width {
        return joined_colored;
    }
    format!("{}{}", " ".repeat(width-visual_len), joined_colored)
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
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
impl TryFrom<&str> for Direction {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Horizontal" => Ok(Direction::Horizontal),
            "Vertical"   => Ok(Direction::Vertical),
            _            => Err("Not a valid Direction value")
        }
    }
}

pub fn is_a_tty<T: AsRawFd>(handle: T) -> bool {
	extern crate libc;
	let fd = handle.as_raw_fd();
    unsafe { libc::isatty(fd) != 0 }
}

pub fn vec_remove_item<T: PartialEq>(vec: &mut Vec<T>, item: &T) -> Option<T> {
    let pos = vec.iter().position(|x| *x == *item)?;
    Some(vec.remove(pos))
}

