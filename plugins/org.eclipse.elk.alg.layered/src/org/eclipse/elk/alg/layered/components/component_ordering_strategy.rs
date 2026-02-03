#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum ComponentOrderingStrategy {
    None,
    InsidePortSideGroups,
    GroupModelOrder,
    ModelOrder,
}
