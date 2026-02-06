#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum GraphCompactionStrategy {
    #[default]
    None,
    Left,
    Right,
    LeftRightConstraintLocking,
    LeftRightConnectionLocking,
    EdgeLength,
}
