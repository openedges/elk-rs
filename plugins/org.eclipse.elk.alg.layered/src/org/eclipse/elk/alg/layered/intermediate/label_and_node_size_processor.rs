use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphUtil, LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

pub struct LabelAndNodeSizeProcessor;

impl Default for LabelAndNodeSizeProcessor {
    fn default() -> Self {
        LabelAndNodeSizeProcessor
    }
}

impl ILayoutProcessor<LGraph> for LabelAndNodeSizeProcessor {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Node and Port Label Placement and Node Sizing", 1.0);

        let layers = graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| LGraphUtil::to_node_array(layer_guard.nodes()))
                .unwrap_or_default();
            for node in nodes {
                place_ports_on_node(&node);
            }
        }

        monitor.done();
    }
}

fn place_ports_on_node(node: &LNodeRef) {
    let (node_type, node_size, port_constraints) = match node.lock() {
        Ok(mut node_guard) => (
            node_guard.node_type(),
            *node_guard.shape().size_ref(),
            node_guard
                .get_property(LayeredOptions::PORT_CONSTRAINTS)
                .unwrap_or(PortConstraints::Undefined),
        ),
        Err(_) => return,
    };

    if node_type != NodeType::Normal {
        return;
    }
    if port_constraints.is_pos_fixed() || port_constraints.is_ratio_fixed() {
        adjust_ports_on_side(node, PortSide::North, node_size.x, node_size.y);
        adjust_ports_on_side(node, PortSide::South, node_size.x, node_size.y);
        adjust_ports_on_side(node, PortSide::East, node_size.x, node_size.y);
        adjust_ports_on_side(node, PortSide::West, node_size.x, node_size.y);
        return;
    }

    place_ports_on_side(node, PortSide::North, node_size.x, node_size.y);
    place_ports_on_side(node, PortSide::South, node_size.x, node_size.y);
    place_ports_on_side(node, PortSide::East, node_size.x, node_size.y);
    place_ports_on_side(node, PortSide::West, node_size.x, node_size.y);
}

fn place_ports_on_side(node: &LNodeRef, side: PortSide, width: f64, height: f64) {
    let ports = node
        .lock()
        .ok()
        .map(|mut node_guard| node_guard.port_side_view(side))
        .unwrap_or_default();
    let count = ports.len();
    if count == 0 {
        return;
    }

    let length = match side {
        PortSide::North | PortSide::South => width,
        PortSide::East | PortSide::West => height,
        _ => return,
    };

    let step = if count == 1 {
        length / 2.0
    } else {
        length / (count as f64 + 1.0)
    };

    let ordered_ports: Vec<_> = match side {
        PortSide::South | PortSide::West => ports.iter().rev().collect(),
        _ => ports.iter().collect(),
    };

    for (index, port) in ordered_ports.iter().enumerate() {
        let offset = if count == 1 {
            step
        } else {
            step * (index as f64 + 1.0)
        };
        if let Ok(mut port_guard) = port.lock() {
            let port_size = *port_guard.shape().size_ref();
            let pos = port_guard.shape().position();
            let axis_size = match side {
                PortSide::North | PortSide::South => port_size.x,
                PortSide::East | PortSide::West => port_size.y,
                _ => 0.0,
            };
            match side {
                PortSide::North => {
                    pos.x = offset - axis_size / 2.0;
                    pos.y = -port_size.y;
                }
                PortSide::South => {
                    pos.x = offset - axis_size / 2.0;
                    pos.y = height;
                }
                PortSide::East => {
                    pos.x = width;
                    pos.y = offset - axis_size / 2.0;
                }
                PortSide::West => {
                    pos.x = -port_size.x;
                    pos.y = offset - axis_size / 2.0;
                }
                _ => {}
            }
        }
    }
}

fn adjust_ports_on_side(node: &LNodeRef, side: PortSide, width: f64, height: f64) {
    let ports = node
        .lock()
        .ok()
        .map(|mut node_guard| node_guard.port_side_view(side))
        .unwrap_or_default();
    if ports.is_empty() {
        return;
    }

    for port in ports {
        if let Ok(mut port_guard) = port.lock() {
            let port_size = *port_guard.shape().size_ref();
            let pos = port_guard.shape().position();
            match side {
                PortSide::North => {
                    pos.y = -port_size.y;
                }
                PortSide::South => {
                    pos.y = height;
                }
                PortSide::East => {
                    pos.x = width;
                }
                PortSide::West => {
                    pos.x = -port_size.x;
                }
                _ => {}
            }
        }
    }
}
