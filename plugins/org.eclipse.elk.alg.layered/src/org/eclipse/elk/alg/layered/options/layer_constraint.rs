#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum LayerConstraint {
    None,
    First,
    FirstSeparate,
    Last,
    LastSeparate,
}
