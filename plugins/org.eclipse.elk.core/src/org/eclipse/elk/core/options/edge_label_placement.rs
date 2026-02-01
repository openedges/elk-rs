#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum EdgeLabelPlacement {
    Center,
    Head,
    Tail,
}

impl EdgeLabelPlacement {
    pub fn is_end_label_placement(self) -> bool {
        matches!(self, EdgeLabelPlacement::Head | EdgeLabelPlacement::Tail)
    }
}
