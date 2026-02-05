#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum RootSelection {
    Fixed,
    CenterNode,
}

impl Default for RootSelection {
    fn default() -> Self {
        RootSelection::CenterNode
    }
}
