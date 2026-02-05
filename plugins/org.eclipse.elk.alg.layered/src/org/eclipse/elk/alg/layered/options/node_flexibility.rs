#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum NodeFlexibility {
    #[default]
    None,
    PortPosition,
    NodeSizeWhereSpacePermits,
    NodeSize,
}

impl NodeFlexibility {
    pub fn is_flexible_size(self) -> bool {
        matches!(self, NodeFlexibility::NodeSize)
    }

    pub fn is_flexible_size_where_space_permits(self) -> bool {
        matches!(
            self,
            NodeFlexibility::NodeSizeWhereSpacePermits | NodeFlexibility::NodeSize
        )
    }

    pub fn is_flexible_ports(self) -> bool {
        matches!(
            self,
            NodeFlexibility::PortPosition
                | NodeFlexibility::NodeSizeWhereSpacePermits
                | NodeFlexibility::NodeSize
        )
    }
}
