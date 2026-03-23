use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LNode, LNodeRef, LPort, LayerRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions, Origin};
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

pub struct LongEdgeSplitter;

impl ILayoutProcessor<LGraph> for LongEdgeSplitter {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Edge splitting", 1.0);

        let layers = layered_graph.layers().clone();
        if layers.len() <= 2 {
            monitor.done();
            return;
        }

        for layer_index in 0..(layers.len() - 1) {
            let layer = layers[layer_index].clone();
            let next_layer = layers[layer_index + 1].clone();
            let nodes = layer
                .lock().nodes().clone();

            for node in nodes {
                let ports = node
                    .lock().ports().clone();
                for port in ports {
                    let outgoing = port
                        .lock().outgoing_edges().clone();
                    for edge in outgoing {
                        let target_layer_index = target_layer_index(&edge, &layers);
                        if target_layer_index != layer_index
                            && target_layer_index != layer_index + 1
                        {
                            trace_long_edge_split("before", layer_index, target_layer_index, &edge);
                            let dummy = create_dummy_node(&next_layer, &edge);
                            Self::split_edge(&edge, &dummy);
                            trace_long_edge_split("after", layer_index, target_layer_index, &edge);
                        }
                    }
                }
            }
        }

        monitor.done();
    }
}

impl LongEdgeSplitter {
    pub fn split_edge(edge: &LEdgeRef, dummy_node: &LNodeRef) -> LEdgeRef {
        let old_edge_target = edge.lock().target();
        let mut thickness = {
            let mut edge_guard = edge.lock();
            if edge_guard
                .graph_element()
                .properties()
                .has_property(CoreOptions::EDGE_THICKNESS)
            {
                edge_guard.get_property(CoreOptions::EDGE_THICKNESS)
            } else {
                None
            }
        }
        .unwrap_or(1.0);
        if thickness < 0.0 {
            thickness = 0.0;
            {
                let mut edge_guard = edge.lock();
                edge_guard.set_property(CoreOptions::EDGE_THICKNESS, Some(thickness));
            }
        }

        {
            let mut dummy_guard = dummy_node.lock();
            dummy_guard.shape().size().y = thickness;
        }
        let port_pos = (thickness / 2.0).floor();

        let dummy_input = LPort::new();
        {
            let mut input_guard = dummy_input.lock();
            input_guard.set_side(PortSide::West);
            input_guard.shape().position().y = port_pos;
        }
        LPort::set_node(&dummy_input, Some(dummy_node.clone()));

        let dummy_output = LPort::new();
        {
            let mut output_guard = dummy_output.lock();
            output_guard.set_side(PortSide::East);
            output_guard.shape().position().y = port_pos;
        }
        LPort::set_node(&dummy_output, Some(dummy_node.clone()));

        LEdge::set_target(edge, Some(dummy_input));

        let dummy_edge = LEdge::new();
        {
            let mut new_edge = dummy_edge.lock();
            let mut old_edge = edge.lock();
            new_edge
                .graph_element()
                .properties_mut()
                .copy_properties(old_edge.graph_element().properties());
            new_edge.set_property(LayeredOptions::JUNCTION_POINTS, None::<KVectorChain>);
        }

        LEdge::set_source(&dummy_edge, Some(dummy_output));
        LEdge::set_target(&dummy_edge, old_edge_target);

        set_dummy_node_properties(dummy_node, edge, &dummy_edge);
        move_head_labels(edge, &dummy_edge);
        dummy_edge
    }
}

fn create_dummy_node(target_layer: &LayerRef, edge_to_split: &LEdgeRef) -> LNodeRef {
    let graph = target_layer
        .lock().graph()
        .unwrap_or_default();
    let dummy = LNode::new(&graph);
    {
        let mut dummy_guard = dummy.lock();
        dummy_guard.set_node_type(NodeType::LongEdge);
        dummy_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LEdge(edge_to_split.clone())),
        );
        dummy_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedPos),
        );
    }
    LNode::set_layer(&dummy, Some(target_layer.clone()));
    dummy
}

fn target_layer_index(edge: &LEdgeRef, layers: &[LayerRef]) -> usize {
    let target_layer = edge
        .lock().target()
        .and_then(|port| port.lock().node())
        .and_then(|node| node.lock().layer());
    let Some(target_layer) = target_layer else {
        return 0;
    };
    layers
        .iter()
        .position(|layer| Arc::ptr_eq(layer, &target_layer))
        .unwrap_or(0)
}

fn move_head_labels(old_edge: &LEdgeRef, new_edge: &LEdgeRef) {
    let labels = old_edge
        .lock().labels().clone();
    for label in labels {
        let placement = {
            let mut label_guard = label.lock();
            if label_guard
                .shape()
                .graph_element()
                .properties()
                .has_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
            {
                label_guard.get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
            } else {
                None
            }
        }
        .unwrap_or(EdgeLabelPlacement::Center);
        if placement != EdgeLabelPlacement::Head {
            continue;
        }

        {
            let mut old_edge_guard = old_edge.lock();
            old_edge_guard
                .labels_mut()
                .retain(|candidate| !Arc::ptr_eq(candidate, &label));
        }
        {
            let mut new_edge_guard = new_edge.lock();
            new_edge_guard.labels_mut().push(label.clone());
        }
        {
            let mut label_guard = label.lock();
            if !label_guard
                .shape()
                .graph_element()
                .properties()
                .has_property(InternalProperties::END_LABEL_EDGE)
            {
                label_guard
                    .set_property(InternalProperties::END_LABEL_EDGE, Some(old_edge.clone()));
            }
        }
    }
}

fn set_dummy_node_properties(dummy_node: &LNodeRef, in_edge: &LEdgeRef, out_edge: &LEdgeRef) {
    let in_edge_source = in_edge
        .lock().source();
    let out_edge_target = out_edge
        .lock().target();

    let in_edge_source_node = in_edge_source
        .as_ref()
        .and_then(|port| port.lock().node());
    let out_edge_target_node = out_edge_target
        .as_ref()
        .and_then(|port| port.lock().node());

    let in_source_type = in_edge_source_node
        .as_ref()
        .map(|node| node.lock().node_type());
    let out_target_type = out_edge_target_node
        .as_ref()
        .map(|node| node.lock().node_type());

    if in_source_type == Some(NodeType::LongEdge) {
        if let Some(in_source_node) = in_edge_source_node {
            {
                let (mut dummy_guard, source_guard) =
                    (dummy_node.lock(), in_source_node.lock());
                dummy_guard.set_property(
                    InternalProperties::LONG_EDGE_SOURCE,
                    source_guard.get_property(InternalProperties::LONG_EDGE_SOURCE),
                );
                dummy_guard.set_property(
                    InternalProperties::LONG_EDGE_TARGET,
                    source_guard.get_property(InternalProperties::LONG_EDGE_TARGET),
                );
                dummy_guard.set_property(
                    InternalProperties::LONG_EDGE_HAS_LABEL_DUMMIES,
                    source_guard.get_property(InternalProperties::LONG_EDGE_HAS_LABEL_DUMMIES),
                );
            }
        }
    } else if in_source_type == Some(NodeType::Label) {
        if let Some(in_source_node) = in_edge_source_node {
            {
                let (mut dummy_guard, source_guard) =
                    (dummy_node.lock(), in_source_node.lock());
                dummy_guard.set_property(
                    InternalProperties::LONG_EDGE_SOURCE,
                    source_guard.get_property(InternalProperties::LONG_EDGE_SOURCE),
                );
                dummy_guard.set_property(
                    InternalProperties::LONG_EDGE_TARGET,
                    source_guard.get_property(InternalProperties::LONG_EDGE_TARGET),
                );
                dummy_guard
                    .set_property(InternalProperties::LONG_EDGE_HAS_LABEL_DUMMIES, Some(true));
            }
        }
    } else if out_target_type == Some(NodeType::Label) {
        if let Some(out_target_node) = out_edge_target_node {
            {
                let (mut dummy_guard, target_guard) =
                    (dummy_node.lock(), out_target_node.lock());
                dummy_guard.set_property(
                    InternalProperties::LONG_EDGE_SOURCE,
                    target_guard.get_property(InternalProperties::LONG_EDGE_SOURCE),
                );
                dummy_guard.set_property(
                    InternalProperties::LONG_EDGE_TARGET,
                    target_guard.get_property(InternalProperties::LONG_EDGE_TARGET),
                );
                dummy_guard
                    .set_property(InternalProperties::LONG_EDGE_HAS_LABEL_DUMMIES, Some(true));
            }
        }
    } else {
        let mut dummy_guard = dummy_node.lock();
        dummy_guard.set_property(InternalProperties::LONG_EDGE_SOURCE, in_edge_source);
        dummy_guard.set_property(InternalProperties::LONG_EDGE_TARGET, out_edge_target);
    }
}

#[cfg(debug_assertions)]
fn trace_long_edge_split(
    phase: &str,
    layer_index: usize,
    target_layer_index: usize,
    edge: &LEdgeRef,
) {
    if !ElkTrace::global().long_edge_split {
        return;
    }

    let (source_ref, target_ref) = {
        let edge_guard = edge.lock();
        (edge_guard.source(), edge_guard.target())
    };
    let source_desc = source_ref
        .map(|source| source.lock().to_string())
        .unwrap_or_else(|| "<no-source>".to_owned());
    let target_desc = target_ref
        .map(|target| target.lock().to_string())
        .unwrap_or_else(|| "<no-target>".to_owned());

    eprintln!(
        "rust-long-split: phase={} layer={} target_layer={} {} -> {}",
        phase, layer_index, target_layer_index, source_desc, target_desc
    );
}

#[cfg(not(debug_assertions))]
fn trace_long_edge_split(
    _phase: &str,
    _layer_index: usize,
    _target_layer_index: usize,
    _edge: &LEdgeRef,
) {
}
