// vim: set ai et ts=4 sw=4 sts=4:
use std::fmt;
use std::io;
use std::os::unix::io::AsRawFd;
use ansi_term::ANSIString;

pub fn stdout_color(s: &ANSIString) -> String {
    match is_a_tty(io::stdout()) {
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
pub fn lalign_colored(s: &ANSIString, width: usize)
    -> String
{
    let visual_len = s.len(); // ANSIString.len() returns length WITHOUT escape sequences
    if visual_len >= width {
        return stdout_color(s);
    }
    format!("{}{}", stdout_color(s), " ".repeat(width-visual_len))
}
pub fn ralign_joined_coloreds(strs: &Vec<ANSIString>, width: usize)
    -> String
{
    let mut visual_len: usize = strs.iter().map(|ansi_str| ansi_str.len()).sum(); // ANSIString.len() returns length WITHOUT escape sequences
    visual_len += strs.len()-1; // count the spaces that .join(" ") will add

    let joined_colored = strs.iter()
                             .map(|astr| stdout_color(astr))
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

pub fn is_a_tty<T: AsRawFd>(handle: T) -> bool {
	extern crate libc;
	let fd = handle.as_raw_fd();
    unsafe { libc::isatty(fd) != 0 }
}

pub fn vec_remove_item<T: PartialEq>(vec: &mut Vec<T>, item: &T) -> Option<T> {
    let pos = vec.iter().position(|x| *x == *item)?;
    Some(vec.remove(pos))
}
