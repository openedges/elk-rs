#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum HierarchyHandling {
    Inherit,
    IncludeChildren,
    SeparateChildren,
}
