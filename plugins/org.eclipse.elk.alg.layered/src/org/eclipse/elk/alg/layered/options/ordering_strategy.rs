#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum OrderingStrategy {
    None,
    NodesAndEdges,
    PreferEdges,
    PreferNodes,
}
