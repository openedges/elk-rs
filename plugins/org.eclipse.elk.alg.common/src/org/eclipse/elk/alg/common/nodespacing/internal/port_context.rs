use crate::org::eclipse::elk::alg::common::nodespacing::cellsystem::DynLabelCell;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMargin, ElkRectangle, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::PortSide;

/// Data holder class for port-related information, to be passed around to avoid having
/// too much state in the size calculation classes.
///
/// Faithfully ports Java's `PortContext` from
/// `org.eclipse.elk.alg.common.nodespacing.internal`.
///
/// Since we want to keep `NodeContext` non-generic (all adapter data is extracted at
/// construction time), `PortContext` stores extracted port data directly rather than
/// holding a reference to a `PortAdapter`.
pub struct PortContext {
    /// Port size (extracted from adapter).
    pub port_size: KVector,
    /// Port side.
    pub port_side: PortSide,
    /// Port's volatile ID (for sorting).
    pub volatile_id: i32,
    /// Port's initial position, to be modified by the algorithm and possibly applied later.
    pub port_position: KVector,
    /// Whether the port's labels need to be placed next to the port.
    pub labels_next_to_port: bool,
    /// Port border offset.
    pub port_border_offset: f64,
    /// Whether the port has the border offset property explicitly set.
    pub has_port_border_offset: bool,
    /// Whether this port has compound connections.
    pub has_compound_connections: bool,
    /// Margin around the port to assume when placing the port. If node labels are taken
    /// into consideration, this will include the label cell. When placing the ports, this
    /// is the size the port will be assumed to have.
    pub port_margin: ElkMargin,
    /// The cell for port labels (set by InsidePortLabelCellCreator / PortContextCreator).
    pub port_label_cell: Option<DynLabelCell>,
    /// Sizes of port labels (extracted from adapter).
    pub label_sizes: Vec<KVector>,
    /// Positions of port labels (extracted from adapter, for fixed label bounds).
    pub label_positions: Vec<KVector>,
    /// The port ratio or position (for fixed ratio placement).
    pub port_ratio_or_position: f64,
}

impl PortContext {
    /// Creates a new port context with the given extracted data.
    pub fn new(
        port_size: KVector,
        port_side: PortSide,
        volatile_id: i32,
        port_position: KVector,
        port_border_offset: f64,
        has_port_border_offset: bool,
        has_compound_connections: bool,
    ) -> Self {
        PortContext {
            port_size,
            port_side,
            volatile_id,
            port_position,
            labels_next_to_port: false,
            port_border_offset,
            has_port_border_offset,
            has_compound_connections,
            port_margin: ElkMargin::new(),
            port_label_cell: None,
            label_sizes: Vec::new(),
            label_positions: Vec::new(),
            port_ratio_or_position: 0.0,
        }
    }

    /// Computes the bounding box of this port's labels (matching Java's
    /// `ElkUtil.getLabelsBounds`). Returns the bounds relative to the port's origin.
    pub fn get_labels_bounds(&self) -> ElkRectangle {
        let mut bounds = ElkRectangle::new();
        let mut first = true;

        for (pos, size) in self.label_positions.iter().zip(self.label_sizes.iter()) {
            if first {
                bounds.x = pos.x;
                bounds.y = pos.y;
                bounds.width = size.x;
                bounds.height = size.y;
                first = false;
            } else {
                let right = (bounds.x + bounds.width).max(pos.x + size.x);
                let bottom = (bounds.y + bounds.height).max(pos.y + size.y);
                bounds.x = bounds.x.min(pos.x);
                bounds.y = bounds.y.min(pos.y);
                bounds.width = right - bounds.x;
                bounds.height = bottom - bounds.y;
            }
        }

        bounds
    }
}
