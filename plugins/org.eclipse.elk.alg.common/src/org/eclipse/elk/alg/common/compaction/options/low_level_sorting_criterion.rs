#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum LowLevelSortingCriterion {
    BySize,
    BySizeAndShape,
}

impl Default for LowLevelSortingCriterion {
    fn default() -> Self {
        LowLevelSortingCriterion::BySizeAndShape
    }
}
