#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum EdgeRoutingStrategy {
    Straight,
    Bend,
}

impl EdgeRoutingStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            EdgeRoutingStrategy::Straight => 0,
            EdgeRoutingStrategy::Bend => 1,
        }
    }
}

impl Default for EdgeRoutingStrategy {
    fn default() -> Self {
        EdgeRoutingStrategy::Straight
    }
}
