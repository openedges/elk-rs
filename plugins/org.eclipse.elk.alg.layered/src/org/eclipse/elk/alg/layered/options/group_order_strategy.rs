#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum GroupOrderStrategy {
    OnlyWithinGroup,
    ModelOrder,
    Enforced,
}
