#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum HighLevelSortingCriterion {
    NumOfExternalSidesThanNumOfExtensionsLast,
    CornerCasesThanSingleSideLast,
}

impl Default for HighLevelSortingCriterion {
    fn default() -> Self {
        HighLevelSortingCriterion::NumOfExternalSidesThanNumOfExtensionsLast
    }
}
