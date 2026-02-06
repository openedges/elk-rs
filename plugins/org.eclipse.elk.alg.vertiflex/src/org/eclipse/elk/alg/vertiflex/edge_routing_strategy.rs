#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum EdgeRoutingStrategy {
    #[default]
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
