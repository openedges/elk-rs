#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum EdgeType {
    None,
    Directed,
    Undirected,
    Association,
    Generalization,
    Dependency,
}
