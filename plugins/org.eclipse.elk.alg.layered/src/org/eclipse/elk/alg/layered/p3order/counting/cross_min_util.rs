use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, LPortRef};

pub fn in_north_south_east_west_order(node: &LNodeRef, side: PortSide) -> Vec<LPortRef> {
    let ports = node
        .lock()
        .ok()
        .map(|mut node_guard| node_guard.port_side_view(side))
        .unwrap_or_default();
    match side {
        PortSide::East | PortSide::North => ports,
        PortSide::South | PortSide::West => ports.into_iter().rev().collect(),
        _ => Vec::new(),
    }
}
