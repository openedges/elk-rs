#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum InLayerConstraint {
    None,
    Top,
    Bottom,
}
