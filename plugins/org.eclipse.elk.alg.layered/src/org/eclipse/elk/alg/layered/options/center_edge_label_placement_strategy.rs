#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum CenterEdgeLabelPlacementStrategy {
    MedianLayer,
    TailLayer,
    HeadLayer,
    SpaceEfficientLayer,
    WidestLayer,
    CenterLayer,
}

impl CenterEdgeLabelPlacementStrategy {
    pub fn uses_label_size_information(self) -> bool {
        matches!(
            self,
            CenterEdgeLabelPlacementStrategy::WidestLayer
                | CenterEdgeLabelPlacementStrategy::CenterLayer
                | CenterEdgeLabelPlacementStrategy::SpaceEfficientLayer
        )
    }
}
