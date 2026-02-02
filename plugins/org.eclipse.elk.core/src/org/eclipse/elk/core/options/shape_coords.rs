#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum ShapeCoords {
    Inherit,
    Parent,
    Root,
}
