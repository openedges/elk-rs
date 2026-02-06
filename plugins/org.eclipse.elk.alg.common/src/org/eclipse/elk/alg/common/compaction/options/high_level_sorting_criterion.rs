#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum HighLevelSortingCriterion {
    #[default]
    NumOfExternalSidesThanNumOfExtensionsLast,
    CornerCasesThanSingleSideLast,
}
