#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum EdgeCoords {
    Inherit,
    Container,
    Parent,
    Root,
}
