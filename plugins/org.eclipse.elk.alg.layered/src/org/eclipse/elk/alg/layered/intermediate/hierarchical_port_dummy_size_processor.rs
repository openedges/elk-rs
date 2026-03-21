use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::alignment::Alignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};

pub struct HierarchicalPortDummySizeProcessor;

impl ILayoutProcessor<LGraph> for HierarchicalPortDummySizeProcessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Hierarchical port dummy size processing", 1.0);

        let edge_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_EDGE_BETWEEN_LAYERS)
            .unwrap_or(0.0);
        let delta = edge_spacing * 2.0;

        for layer in layered_graph.layers().clone() {
            let nodes = layer
                .lock_ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            let mut northern_dummies: Vec<LNodeRef> = Vec::new();
            let mut southern_dummies: Vec<LNodeRef> = Vec::new();

            for node in nodes {
                let (node_type, side) = node
                    .lock_ok()
                    .map(|mut node_guard| {
                        (
                            node_guard.node_type(),
                            node_guard
                                .get_property(InternalProperties::EXT_PORT_SIDE)
                                .unwrap_or(PortSide::Undefined),
                        )
                    })
                    .unwrap_or((NodeType::Normal, PortSide::Undefined));

                if node_type == NodeType::ExternalPort {
                    if side == PortSide::North {
                        northern_dummies.push(node);
                    } else if side == PortSide::South {
                        southern_dummies.push(node);
                    }
                }
            }

            set_widths(&northern_dummies, true, delta);
            set_widths(&southern_dummies, false, delta);
        }

        monitor.done();
    }
}

fn set_widths(nodes: &[LNodeRef], top_down: bool, delta: f64) {
    if nodes.is_empty() {
        return;
    }

    let mut current_width = if top_down {
        0.0
    } else {
        delta * (nodes.len().saturating_sub(1) as f64)
    };
    let step = if top_down { delta } else { -delta };

    for node in nodes {
        let east_ports = if let Some(mut node_guard) = node.lock_ok() {
            node_guard.set_property(LayeredOptions::ALIGNMENT, Some(Alignment::Center));
            node_guard.shape().size().x = current_width;
            node_guard.ports_by_side(PortSide::East)
        } else {
            Vec::new()
        };

        for port in east_ports {
            if let Some(mut port_guard) = port.lock_ok() {
                port_guard.shape().position().x = current_width;
            }
        }

        current_width += step;
    }
}
