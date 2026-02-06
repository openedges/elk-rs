#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum RootSelection {
    Fixed,
    #[default]
    CenterNode,
}
