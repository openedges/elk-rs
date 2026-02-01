#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum PortConstraints {
    Undefined,
    Free,
    FixedSide,
    FixedOrder,
    FixedRatio,
    FixedPos,
}

impl PortConstraints {
    pub fn is_pos_fixed(self) -> bool {
        self == PortConstraints::FixedPos
    }

    pub fn is_ratio_fixed(self) -> bool {
        self == PortConstraints::FixedRatio
    }

    pub fn is_order_fixed(self) -> bool {
        matches!(
            self,
            PortConstraints::FixedOrder | PortConstraints::FixedRatio | PortConstraints::FixedPos
        )
    }

    pub fn is_side_fixed(self) -> bool {
        !matches!(self, PortConstraints::Free | PortConstraints::Undefined)
    }
}
