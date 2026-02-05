#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum NodePromotionStrategy {
    None,
    Nikolov,
    NikolovPixel,
    NikolovImproved,
    NikolovImprovedPixel,
    DummynodePercentage,
    NodecountPercentage,
    NoBoundary,
    ModelOrderLeftToRight,
    ModelOrderRightToLeft,
}

impl Default for NodePromotionStrategy {
    fn default() -> Self {
        NodePromotionStrategy::None
    }
}
