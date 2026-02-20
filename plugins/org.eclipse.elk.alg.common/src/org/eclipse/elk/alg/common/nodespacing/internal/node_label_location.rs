use crate::org::eclipse::elk::alg::common::nodespacing::cellsystem::{
    ContainerArea, HorizontalLabelAlignment, VerticalLabelAlignment,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{NodeLabelPlacement, PortSide};
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

/// Enumeration over all possible label placements and associated things.
///
/// Faithfully ports Java's `NodeLabelLocation` enum from
/// `org.eclipse.elk.alg.common.nodespacing.internal`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeLabelLocation {
    /// Outside top left.
    OutTL,
    /// Outside top center.
    OutTC,
    /// Outside top right.
    OutTR,
    /// Outside bottom left.
    OutBL,
    /// Outside bottom center.
    OutBC,
    /// Outside bottom right.
    OutBR,
    /// Outside left top.
    OutLT,
    /// Outside left center.
    OutLC,
    /// Outside left bottom.
    OutLB,
    /// Outside right top.
    OutRT,
    /// Outside right center.
    OutRC,
    /// Outside right bottom.
    OutRB,
    /// Inside top left.
    InTL,
    /// Inside top center.
    InTC,
    /// Inside top right.
    InTR,
    /// Inside center left.
    InCL,
    /// Inside center center.
    InCC,
    /// Inside center right.
    InCR,
    /// Inside bottom left.
    InBL,
    /// Inside bottom center.
    InBC,
    /// Inside bottom right.
    InBR,
    /// Undefined or not decidable.
    Undefined,
}

/// Static data for each variant: (horizontal_alignment, vertical_alignment, row, column).
/// `None` values correspond to Java's `null` for the UNDEFINED variant.
struct LocationData {
    horizontal_alignment: Option<HorizontalLabelAlignment>,
    vertical_alignment: Option<VerticalLabelAlignment>,
    container_row: Option<ContainerArea>,
    container_column: Option<ContainerArea>,
}

impl NodeLabelLocation {
    /// Returns an iterator over all defined variants (excluding Undefined).
    pub fn all_defined() -> &'static [NodeLabelLocation; 21] {
        &Self::ALL_DEFINED
    }

    /// All defined variants (excluding Undefined), used for iteration.
    const ALL_DEFINED: [NodeLabelLocation; 21] = [
        NodeLabelLocation::OutTL,
        NodeLabelLocation::OutTC,
        NodeLabelLocation::OutTR,
        NodeLabelLocation::OutBL,
        NodeLabelLocation::OutBC,
        NodeLabelLocation::OutBR,
        NodeLabelLocation::OutLT,
        NodeLabelLocation::OutLC,
        NodeLabelLocation::OutLB,
        NodeLabelLocation::OutRT,
        NodeLabelLocation::OutRC,
        NodeLabelLocation::OutRB,
        NodeLabelLocation::InTL,
        NodeLabelLocation::InTC,
        NodeLabelLocation::InTR,
        NodeLabelLocation::InCL,
        NodeLabelLocation::InCC,
        NodeLabelLocation::InCR,
        NodeLabelLocation::InBL,
        NodeLabelLocation::InBC,
        NodeLabelLocation::InBR,
    ];

    fn data(&self) -> LocationData {
        use ContainerArea as CA;
        use HorizontalLabelAlignment as HA;
        use VerticalLabelAlignment as VA;

        match self {
            // Outside top
            NodeLabelLocation::OutTL => LocationData {
                horizontal_alignment: Some(HA::Left),
                vertical_alignment: Some(VA::Bottom),
                container_row: Some(CA::Begin),
                container_column: Some(CA::Begin),
            },
            NodeLabelLocation::OutTC => LocationData {
                horizontal_alignment: Some(HA::Center),
                vertical_alignment: Some(VA::Bottom),
                container_row: Some(CA::Begin),
                container_column: Some(CA::Center),
            },
            NodeLabelLocation::OutTR => LocationData {
                horizontal_alignment: Some(HA::Right),
                vertical_alignment: Some(VA::Bottom),
                container_row: Some(CA::Begin),
                container_column: Some(CA::End),
            },
            // Outside bottom
            NodeLabelLocation::OutBL => LocationData {
                horizontal_alignment: Some(HA::Left),
                vertical_alignment: Some(VA::Top),
                container_row: Some(CA::End),
                container_column: Some(CA::Begin),
            },
            NodeLabelLocation::OutBC => LocationData {
                horizontal_alignment: Some(HA::Center),
                vertical_alignment: Some(VA::Top),
                container_row: Some(CA::End),
                container_column: Some(CA::Center),
            },
            NodeLabelLocation::OutBR => LocationData {
                horizontal_alignment: Some(HA::Right),
                vertical_alignment: Some(VA::Top),
                container_row: Some(CA::End),
                container_column: Some(CA::End),
            },
            // Outside left (HPriority variants)
            NodeLabelLocation::OutLT => LocationData {
                horizontal_alignment: Some(HA::Right),
                vertical_alignment: Some(VA::Top),
                container_row: Some(CA::Begin),
                container_column: Some(CA::Begin),
            },
            NodeLabelLocation::OutLC => LocationData {
                horizontal_alignment: Some(HA::Right),
                vertical_alignment: Some(VA::Center),
                container_row: Some(CA::Center),
                container_column: Some(CA::Begin),
            },
            NodeLabelLocation::OutLB => LocationData {
                horizontal_alignment: Some(HA::Right),
                vertical_alignment: Some(VA::Bottom),
                container_row: Some(CA::End),
                container_column: Some(CA::Begin),
            },
            // Outside right (HPriority variants)
            NodeLabelLocation::OutRT => LocationData {
                horizontal_alignment: Some(HA::Left),
                vertical_alignment: Some(VA::Top),
                container_row: Some(CA::Begin),
                container_column: Some(CA::End),
            },
            NodeLabelLocation::OutRC => LocationData {
                horizontal_alignment: Some(HA::Left),
                vertical_alignment: Some(VA::Center),
                container_row: Some(CA::Center),
                container_column: Some(CA::End),
            },
            NodeLabelLocation::OutRB => LocationData {
                horizontal_alignment: Some(HA::Left),
                vertical_alignment: Some(VA::Bottom),
                container_row: Some(CA::End),
                container_column: Some(CA::End),
            },
            // Inside top
            NodeLabelLocation::InTL => LocationData {
                horizontal_alignment: Some(HA::Left),
                vertical_alignment: Some(VA::Top),
                container_row: Some(CA::Begin),
                container_column: Some(CA::Begin),
            },
            NodeLabelLocation::InTC => LocationData {
                horizontal_alignment: Some(HA::Center),
                vertical_alignment: Some(VA::Top),
                container_row: Some(CA::Begin),
                container_column: Some(CA::Center),
            },
            NodeLabelLocation::InTR => LocationData {
                horizontal_alignment: Some(HA::Right),
                vertical_alignment: Some(VA::Top),
                container_row: Some(CA::Begin),
                container_column: Some(CA::End),
            },
            // Inside center
            NodeLabelLocation::InCL => LocationData {
                horizontal_alignment: Some(HA::Left),
                vertical_alignment: Some(VA::Center),
                container_row: Some(CA::Center),
                container_column: Some(CA::Begin),
            },
            NodeLabelLocation::InCC => LocationData {
                horizontal_alignment: Some(HA::Center),
                vertical_alignment: Some(VA::Center),
                container_row: Some(CA::Center),
                container_column: Some(CA::Center),
            },
            NodeLabelLocation::InCR => LocationData {
                horizontal_alignment: Some(HA::Right),
                vertical_alignment: Some(VA::Center),
                container_row: Some(CA::Center),
                container_column: Some(CA::End),
            },
            // Inside bottom
            NodeLabelLocation::InBL => LocationData {
                horizontal_alignment: Some(HA::Left),
                vertical_alignment: Some(VA::Bottom),
                container_row: Some(CA::End),
                container_column: Some(CA::Begin),
            },
            NodeLabelLocation::InBC => LocationData {
                horizontal_alignment: Some(HA::Center),
                vertical_alignment: Some(VA::Bottom),
                container_row: Some(CA::End),
                container_column: Some(CA::Center),
            },
            NodeLabelLocation::InBR => LocationData {
                horizontal_alignment: Some(HA::Right),
                vertical_alignment: Some(VA::Bottom),
                container_row: Some(CA::End),
                container_column: Some(CA::End),
            },
            // Undefined
            NodeLabelLocation::Undefined => LocationData {
                horizontal_alignment: None,
                vertical_alignment: None,
                container_row: None,
                container_column: None,
            },
        }
    }

    /// Returns the matching placement sets for this location.
    /// Each variant has 1 or 2 `EnumSet<NodeLabelPlacement>` that map to it.
    fn assigned_placements(&self) -> Vec<EnumSet<NodeLabelPlacement>> {
        use NodeLabelPlacement::*;

        match self {
            // Outside top
            NodeLabelLocation::OutTL => vec![EnumSet::of(&[Outside, VTop, HLeft])],
            NodeLabelLocation::OutTC => vec![
                EnumSet::of(&[Outside, VTop, HCenter]),
                EnumSet::of(&[Outside, VTop, HCenter, HPriority]),
            ],
            NodeLabelLocation::OutTR => vec![EnumSet::of(&[Outside, VTop, HRight])],
            // Outside bottom
            NodeLabelLocation::OutBL => vec![EnumSet::of(&[Outside, VBottom, HLeft])],
            NodeLabelLocation::OutBC => vec![
                EnumSet::of(&[Outside, VBottom, HCenter]),
                EnumSet::of(&[Outside, VBottom, HCenter, HPriority]),
            ],
            NodeLabelLocation::OutBR => vec![EnumSet::of(&[Outside, VBottom, HRight])],
            // Outside left (only HPriority)
            NodeLabelLocation::OutLT => {
                vec![EnumSet::of(&[Outside, HLeft, VTop, HPriority])]
            }
            NodeLabelLocation::OutLC => vec![
                EnumSet::of(&[Outside, HLeft, VCenter]),
                EnumSet::of(&[Outside, HLeft, VCenter, HPriority]),
            ],
            NodeLabelLocation::OutLB => {
                vec![EnumSet::of(&[Outside, HLeft, VBottom, HPriority])]
            }
            // Outside right (only HPriority)
            NodeLabelLocation::OutRT => {
                vec![EnumSet::of(&[Outside, HRight, VTop, HPriority])]
            }
            NodeLabelLocation::OutRC => vec![
                EnumSet::of(&[Outside, HRight, VCenter]),
                EnumSet::of(&[Outside, HRight, VCenter, HPriority]),
            ],
            NodeLabelLocation::OutRB => {
                vec![EnumSet::of(&[Outside, HRight, VBottom, HPriority])]
            }
            // Inside (all have both with and without HPriority)
            NodeLabelLocation::InTL => vec![
                EnumSet::of(&[Inside, VTop, HLeft]),
                EnumSet::of(&[Inside, VTop, HLeft, HPriority]),
            ],
            NodeLabelLocation::InTC => vec![
                EnumSet::of(&[Inside, VTop, HCenter]),
                EnumSet::of(&[Inside, VTop, HCenter, HPriority]),
            ],
            NodeLabelLocation::InTR => vec![
                EnumSet::of(&[Inside, VTop, HRight]),
                EnumSet::of(&[Inside, VTop, HRight, HPriority]),
            ],
            NodeLabelLocation::InCL => vec![
                EnumSet::of(&[Inside, VCenter, HLeft]),
                EnumSet::of(&[Inside, VCenter, HLeft, HPriority]),
            ],
            NodeLabelLocation::InCC => vec![
                EnumSet::of(&[Inside, VCenter, HCenter]),
                EnumSet::of(&[Inside, VCenter, HCenter, HPriority]),
            ],
            NodeLabelLocation::InCR => vec![
                EnumSet::of(&[Inside, VCenter, HRight]),
                EnumSet::of(&[Inside, VCenter, HRight, HPriority]),
            ],
            NodeLabelLocation::InBL => vec![
                EnumSet::of(&[Inside, VBottom, HLeft]),
                EnumSet::of(&[Inside, VBottom, HLeft, HPriority]),
            ],
            NodeLabelLocation::InBC => vec![
                EnumSet::of(&[Inside, VBottom, HCenter]),
                EnumSet::of(&[Inside, VBottom, HCenter, HPriority]),
            ],
            NodeLabelLocation::InBR => vec![
                EnumSet::of(&[Inside, VBottom, HRight]),
                EnumSet::of(&[Inside, VBottom, HRight, HPriority]),
            ],
            // Undefined has no assigned placements
            NodeLabelLocation::Undefined => vec![],
        }
    }

    /// Converts a set of `NodeLabelPlacement`s to a `NodeLabelLocation` if possible.
    /// If no valid combination is given, `NodeLabelLocation::Undefined` is returned.
    pub fn from_node_label_placement(
        placement: &EnumSet<NodeLabelPlacement>,
    ) -> NodeLabelLocation {
        for location in &Self::ALL_DEFINED {
            for assigned in location.assigned_placements() {
                if *placement == assigned {
                    return *location;
                }
            }
        }
        NodeLabelLocation::Undefined
    }

    /// Returns the horizontal text alignment for this location.
    pub fn horizontal_alignment(&self) -> Option<HorizontalLabelAlignment> {
        self.data().horizontal_alignment
    }

    /// Returns the vertical text alignment for this location.
    pub fn vertical_alignment(&self) -> Option<VerticalLabelAlignment> {
        self.data().vertical_alignment
    }

    /// Returns the appropriate row in a container cell.
    pub fn container_row(&self) -> Option<ContainerArea> {
        self.data().container_row
    }

    /// Returns the appropriate column in a container cell.
    pub fn container_column(&self) -> Option<ContainerArea> {
        self.data().container_column
    }

    /// Checks whether this location is inside the node or not.
    pub fn is_inside_location(&self) -> bool {
        matches!(
            self,
            NodeLabelLocation::InTL
                | NodeLabelLocation::InTC
                | NodeLabelLocation::InTR
                | NodeLabelLocation::InCL
                | NodeLabelLocation::InCC
                | NodeLabelLocation::InCR
                | NodeLabelLocation::InBL
                | NodeLabelLocation::InBC
                | NodeLabelLocation::InBR
        )
    }

    /// Returns the side of the node an outside node label location corresponds to,
    /// or `PortSide::Undefined` if this location is on the inside or `Undefined`.
    pub fn outside_side(&self) -> PortSide {
        match self {
            NodeLabelLocation::OutTL | NodeLabelLocation::OutTC | NodeLabelLocation::OutTR => {
                PortSide::North
            }
            NodeLabelLocation::OutBL | NodeLabelLocation::OutBC | NodeLabelLocation::OutBR => {
                PortSide::South
            }
            NodeLabelLocation::OutLT | NodeLabelLocation::OutLC | NodeLabelLocation::OutLB => {
                PortSide::West
            }
            NodeLabelLocation::OutRT | NodeLabelLocation::OutRC | NodeLabelLocation::OutRB => {
                PortSide::East
            }
            _ => PortSide::Undefined,
        }
    }
}
