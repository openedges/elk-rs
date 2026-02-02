#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum TopdownNodeTypes {
    ParallelNode,
    HierarchicalNode,
    RootNode,
}
