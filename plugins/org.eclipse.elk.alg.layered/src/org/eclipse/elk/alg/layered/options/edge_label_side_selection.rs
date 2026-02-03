#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum EdgeLabelSideSelection {
    AlwaysUp,
    AlwaysDown,
    DirectionUp,
    DirectionDown,
    SmartUp,
    SmartDown,
}

impl EdgeLabelSideSelection {
    pub fn transpose(self) -> Self {
        match self {
            EdgeLabelSideSelection::AlwaysUp => EdgeLabelSideSelection::AlwaysDown,
            EdgeLabelSideSelection::AlwaysDown => EdgeLabelSideSelection::AlwaysUp,
            EdgeLabelSideSelection::DirectionUp => EdgeLabelSideSelection::DirectionDown,
            EdgeLabelSideSelection::DirectionDown => EdgeLabelSideSelection::DirectionUp,
            EdgeLabelSideSelection::SmartUp => EdgeLabelSideSelection::SmartDown,
            EdgeLabelSideSelection::SmartDown => EdgeLabelSideSelection::SmartUp,
        }
    }
}
