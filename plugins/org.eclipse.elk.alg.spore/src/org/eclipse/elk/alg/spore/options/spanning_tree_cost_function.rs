#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum SpanningTreeCostFunction {
    CenterDistance,
    #[default]
    CircleUnderlap,
    RectangleUnderlap,
    InvertedOverlap,
    MinimumRootDistance,
}
