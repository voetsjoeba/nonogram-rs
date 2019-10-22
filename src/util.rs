// vim: set ai et ts=4 sw=4 sts=4:
use std::fmt;
use std::os::unix::io::AsRawFd;
use ansi_term::ANSIString;

pub fn ralign(s: &str, width: usize) -> String {
    if s.len() >= width {
        return String::from(s);
    }
    format!("{}{}", " ".repeat(width-s.len()), s)
}
pub fn lalign_colored(s: &ANSIString, width: usize)
    -> String
{
    let visual_len = s.len(); // ANSIString.len() returns length WITHOUT escape sequences
    if visual_len >= width {
        return s.to_string() // returns string WITH escape sequences
    }
    format!("{}{}", s.to_string(), " ".repeat(width-visual_len))
}
pub fn ralign_joined_coloreds(strs: &Vec<ANSIString>, width: usize)
    -> String
{
    let mut visual_len: usize = strs.iter().map(|ansi_str| ansi_str.len()).sum(); // ANSIString.len() returns length WITHOUT escape sequences
    visual_len += strs.len()-1; // count the spaces that .join(" ") will add

    let joined_colored = strs.iter().map(|astr| astr.to_string()).collect::<Vec<_>>().join(" ");
    if visual_len >= width {
        return joined_colored;
    }
    format!("{}{}", " ".repeat(width-visual_len), joined_colored)
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

pub fn is_a_tty<T: AsRawFd>(handle: T) -> bool {
	extern crate libc;
	let fd = handle.as_raw_fd();
    unsafe { libc::isatty(fd) != 0 }
}
