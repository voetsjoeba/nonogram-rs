// vim: set ai et ts=4 sw=4 sts=4:
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
