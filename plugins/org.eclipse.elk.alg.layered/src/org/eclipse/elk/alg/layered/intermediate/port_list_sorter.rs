use std::cmp::Ordering;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LPortRef};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

pub struct PortListSorter;

impl Default for PortListSorter {
    fn default() -> Self {
        PortListSorter
    }
}

impl ILayoutProcessor<LGraph> for PortListSorter {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Port list sorting", 1.0);

        let layers = graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            for node in nodes {
                let constraints = node
                    .lock()
                    .ok()
                    .and_then(|mut node_guard| node_guard.get_property(LayeredOptions::PORT_CONSTRAINTS))
                    .unwrap_or(PortConstraints::Undefined);

                if !constraints.is_side_fixed() {
                    continue;
                }

                if let Ok(mut node_guard) = node.lock() {
                    node_guard
                        .ports_mut()
                        .sort_by(|p1, p2| compare_ports(p1, p2, constraints));
                    node_guard.cache_port_sides();
                }
            }
        }

        monitor.done();
    }
}

fn compare_ports(p1: &LPortRef, p2: &LPortRef, constraints: PortConstraints) -> Ordering {
    let side1 = p1
        .lock()
        .ok()
        .map(|port_guard| port_guard.side())
        .unwrap_or(PortSide::Undefined);
    let side2 = p2
        .lock()
        .ok()
        .map(|port_guard| port_guard.side())
        .unwrap_or(PortSide::Undefined);
    let side_cmp = side1.cmp(&side2);
    if side_cmp != Ordering::Equal {
        return side_cmp;
    }

    if !constraints.is_order_fixed() {
        return Ordering::Equal;
    }

    if constraints == PortConstraints::FixedOrder {
        let idx1 = p1
            .lock()
            .ok()
            .and_then(|mut port_guard| port_guard.get_property(LayeredOptions::PORT_INDEX));
        let idx2 = p2
            .lock()
            .ok()
            .and_then(|mut port_guard| port_guard.get_property(LayeredOptions::PORT_INDEX));
        if let (Some(i1), Some(i2)) = (idx1, idx2) {
            if i1 != i2 {
                return i1.cmp(&i2);
            }
        }
    }

    let pos1 = p1
        .lock()
        .ok()
        .map(|mut port_guard| *port_guard.shape().position_ref())
        .unwrap_or_default();
    let pos2 = p2
        .lock()
        .ok()
        .map(|mut port_guard| *port_guard.shape().position_ref())
        .unwrap_or_default();
    match side1 {
        PortSide::North => pos1.x.partial_cmp(&pos2.x).unwrap_or(Ordering::Equal),
        PortSide::East => pos1.y.partial_cmp(&pos2.y).unwrap_or(Ordering::Equal),
        PortSide::South => pos2.x.partial_cmp(&pos1.x).unwrap_or(Ordering::Equal),
        PortSide::West => pos2.y.partial_cmp(&pos1.y).unwrap_or(Ordering::Equal),
        PortSide::Undefined => Ordering::Equal,
    }
}
