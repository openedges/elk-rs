#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub fn is_horizontal(self) -> bool {
        matches!(self, Direction::East | Direction::West)
    }
}
