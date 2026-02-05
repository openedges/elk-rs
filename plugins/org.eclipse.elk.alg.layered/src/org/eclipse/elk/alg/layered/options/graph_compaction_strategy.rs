#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum GraphCompactionStrategy {
    None,
    Left,
    Right,
    LeftRightConstraintLocking,
    LeftRightConnectionLocking,
    EdgeLength,
}

impl Default for GraphCompactionStrategy {
    fn default() -> Self {
        GraphCompactionStrategy::None
    }
}
