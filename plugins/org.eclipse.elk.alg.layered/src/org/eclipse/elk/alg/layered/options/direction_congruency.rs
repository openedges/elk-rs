#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum DirectionCongruency {
    #[default]
    ReadingDirection,
    Rotation,
}
