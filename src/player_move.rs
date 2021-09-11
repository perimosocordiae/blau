use crate::colors::Color;

#[derive(Debug, Clone, Copy)]
pub struct Move {
    pub factory_idx: usize,
    pub color: Color,
    pub working_row: usize,
}

impl Move {
    pub fn is_from_center(&self) -> bool {
        self.factory_idx == 0
    }
    pub fn check_validity(&self) -> Option<String> {
        if !self.color.is_movable() {
            return Some(format!("{:?} tiles are not movable.", self.color));
        }
        if self.working_row > 5 {
            return Some(format!("Cannot move to row {}.", self.working_row));
        }
        None
    }
}

#[test]
fn catches_invalid_color() {
    assert_eq!(
        Move {
            factory_idx: 0,
            color: Color::Start,
            working_row: 1
        }
        .check_validity(),
        Some("Start tiles are not movable.".to_string())
    );
}

#[test]
fn catches_invalid_destination() {
    assert_eq!(
        Move {
            factory_idx: 0,
            color: Color::Blue,
            working_row: 6
        }
        .check_validity(),
        Some("Cannot move to row 6.".to_string())
    );
}
