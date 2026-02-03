#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum WrappingStrategy {
    Off,
    SingleEdge,
    MultiEdge,
}
