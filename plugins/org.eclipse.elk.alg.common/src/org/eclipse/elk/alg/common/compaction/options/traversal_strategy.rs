#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum TraversalStrategy {
    Spiral,
    LineByLine,
    Manhattan,
    Jitter,
    QuadrantsLineByLine,
    QuadrantsManhattan,
    QuadrantsJitter,
    CombineLineByLineManhattan,
    CombineJitterManhattan,
}

impl Default for TraversalStrategy {
    fn default() -> Self {
        TraversalStrategy::QuadrantsLineByLine
    }
}
