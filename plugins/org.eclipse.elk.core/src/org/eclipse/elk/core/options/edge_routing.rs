#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum EdgeRouting {
    Undefined,
    Polyline,
    Orthogonal,
    Splines,
}
