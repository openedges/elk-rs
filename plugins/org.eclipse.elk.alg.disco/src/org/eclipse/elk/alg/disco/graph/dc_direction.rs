#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum DCDirection {
    North,
    East,
    South,
    West,
}

impl DCDirection {
    pub fn is_horizontal(self) -> bool {
        matches!(self, DCDirection::East | DCDirection::West)
    }
}
