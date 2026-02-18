use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LayerRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};

pub struct HierarchicalPortPositionProcessor;

impl ILayoutProcessor<LGraph> for HierarchicalPortPositionProcessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Hierarchical port position processing", 1.0);

        let layers = layered_graph.layers().clone();
        if !layers.is_empty() {
            fix_coordinates(&layers[0], layered_graph);
        }
        if layers.len() > 1 {
            fix_coordinates(&layers[layers.len() - 1], layered_graph);
        }

        monitor.done();
    }
}

fn fix_coordinates(layer: &LayerRef, layered_graph: &mut LGraph) {
    let port_constraints = layered_graph
        .get_property(LayeredOptions::PORT_CONSTRAINTS)
        .unwrap_or(PortConstraints::Undefined);
    if !(port_constraints.is_ratio_fixed() || port_constraints.is_pos_fixed()) {
        return;
    }

    let graph_height = layered_graph.actual_size().y;

    let nodes = layer
        .lock()
        .ok()
        .map(|layer_guard| layer_guard.nodes().clone())
        .unwrap_or_default();

    for node in nodes {
        let (node_type, ext_side, ratio_or_pos, anchor) = node
            .lock()
            .ok()
            .map(|mut node_guard| {
                (
                    node_guard.node_type(),
                    node_guard
                        .get_property(InternalProperties::EXT_PORT_SIDE)
                        .unwrap_or(PortSide::Undefined),
                    node_guard
                        .get_property(InternalProperties::PORT_RATIO_OR_POSITION)
                        .unwrap_or(0.0),
                    node_guard
                        .get_property(LayeredOptions::PORT_ANCHOR)
                        .unwrap_or_default(),
                )
            })
            .unwrap_or((
                NodeType::Normal,
                PortSide::Undefined,
                0.0,
                Default::default(),
            ));

        if node_type != NodeType::ExternalPort {
            continue;
        }
        if ext_side != PortSide::East && ext_side != PortSide::West {
            continue;
        }

        let mut final_y = ratio_or_pos;
        if port_constraints == PortConstraints::FixedRatio {
            final_y *= graph_height;
        }

        if let Ok(mut node_guard) = node.lock() {
            let padding_top = layered_graph.padding_ref().top;
            let offset_y = layered_graph.offset_ref().y;
            node_guard.shape().position().y = final_y - anchor.y - padding_top - offset_y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HierarchicalPortPositionProcessor;
    use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNode, Layer, NodeType};
    use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};
    use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
    use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
    use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
    use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
    use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;

    #[test]
    fn hierarchical_port_position_processor_does_not_deadlock_with_graph_lock() {
        let graph = LGraph::new();
        let layer = Layer::new(&graph);

        if let Ok(mut graph_guard) = graph.lock() {
            graph_guard.layers_mut().push(layer.clone());
            graph_guard.set_property(
                LayeredOptions::PORT_CONSTRAINTS,
                Some(PortConstraints::FixedPos),
            );
            let size = graph_guard.size();
            size.x = 100.0;
            size.y = 100.0;
        }

        let node = LNode::new(&graph);
        LNode::set_layer(&node, Some(layer.clone()));
        if let Ok(mut node_guard) = node.lock() {
            node_guard.set_node_type(NodeType::ExternalPort);
            node_guard.set_property(InternalProperties::EXT_PORT_SIDE, Some(PortSide::East));
            node_guard.set_property(InternalProperties::PORT_RATIO_OR_POSITION, Some(0.5));
            node_guard.set_property(LayeredOptions::PORT_ANCHOR, Some(KVector::new()));
        }
        if let Ok(mut layer_guard) = layer.lock() {
            layer_guard.nodes_mut().push(node);
        }

        let mut processor = HierarchicalPortPositionProcessor;
        if let Ok(mut graph_guard) = graph.lock() {
            let mut monitor = BasicProgressMonitor::new();
            processor.process(&mut graph_guard, &mut monitor);
        };
    }
}
