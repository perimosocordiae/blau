use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    Blue = 0,
    Orange,
    Green,
    Red,
    Purple,
    Start,
    Blank,
}

impl Color {
    pub fn is_movable(&self) -> bool {
        *self != Color::Start && *self != Color::Blank
    }
}

#[test]
fn checks_movable() {
    assert!(Color::Blue.is_movable());
    assert!(!Color::Start.is_movable());
    assert!(!Color::Blank.is_movable());
}

impl TryFrom<usize> for Color {
    type Error = ();
    fn try_from(v: usize) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Color::Blue),
            1 => Ok(Color::Orange),
            2 => Ok(Color::Green),
            3 => Ok(Color::Red),
            4 => Ok(Color::Purple),
            _ => Err(()),
        }
    }
}
