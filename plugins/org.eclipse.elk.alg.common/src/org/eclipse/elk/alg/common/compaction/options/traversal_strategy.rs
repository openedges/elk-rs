#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum TraversalStrategy {
    Spiral,
    LineByLine,
    Manhattan,
    Jitter,
    #[default]
    QuadrantsLineByLine,
    QuadrantsManhattan,
    QuadrantsJitter,
    CombineLineByLineManhattan,
    CombineJitterManhattan,
}
