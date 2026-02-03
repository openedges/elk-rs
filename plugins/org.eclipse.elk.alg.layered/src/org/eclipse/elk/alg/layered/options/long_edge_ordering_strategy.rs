#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum LongEdgeOrderingStrategy {
    DummyNodeOver,
    DummyNodeUnder,
    Equal,
}

impl LongEdgeOrderingStrategy {
    pub fn return_value(self) -> i32 {
        match self {
            LongEdgeOrderingStrategy::DummyNodeOver => i32::MAX,
            LongEdgeOrderingStrategy::DummyNodeUnder => i32::MIN,
            LongEdgeOrderingStrategy::Equal => 0,
        }
    }
}
