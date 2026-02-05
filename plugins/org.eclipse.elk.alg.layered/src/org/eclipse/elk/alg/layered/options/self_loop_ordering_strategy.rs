#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum SelfLoopOrderingStrategy {
    #[default]
    Stacked,
    ReverseStacked,
    Sequenced,
}
