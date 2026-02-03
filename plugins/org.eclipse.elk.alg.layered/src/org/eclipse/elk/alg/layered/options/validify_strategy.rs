#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum ValidifyStrategy {
    No,
    Greedy,
    LookBack,
}
