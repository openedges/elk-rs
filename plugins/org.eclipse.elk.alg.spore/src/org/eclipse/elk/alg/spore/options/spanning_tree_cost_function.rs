#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum SpanningTreeCostFunction {
    CenterDistance,
    CircleUnderlap,
    RectangleUnderlap,
    InvertedOverlap,
    MinimumRootDistance,
}

impl Default for SpanningTreeCostFunction {
    fn default() -> Self {
        SpanningTreeCostFunction::CircleUnderlap
    }
}
