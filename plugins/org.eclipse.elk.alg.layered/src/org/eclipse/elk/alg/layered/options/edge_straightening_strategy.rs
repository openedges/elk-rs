#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum EdgeStraighteningStrategy {
    None,
    #[default]
    ImproveStraightness,
}
