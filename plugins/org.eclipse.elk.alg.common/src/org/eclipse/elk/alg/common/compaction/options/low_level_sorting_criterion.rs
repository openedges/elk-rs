#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum LowLevelSortingCriterion {
    BySize,
    #[default]
    BySizeAndShape,
}
