#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum DrawingDataDescriptor {
    CandidatePositionLastPlacedRight,
    CandidatePositionLastPlacedBelow,
    CandidatePositionWholeDrawingRight,
    CandidatePositionWholeDrawingBelow,
    WholeDrawing,
}
