#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum NodePromotionStrategy {
    #[default]
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
