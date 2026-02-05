#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum SelfLoopDistributionStrategy {
    Equally,
    #[default]
    North,
    NorthSouth,
}
