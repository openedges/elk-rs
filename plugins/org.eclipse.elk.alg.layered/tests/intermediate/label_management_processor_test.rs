
use std::any::Any;
use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use crate::common::issue_support::init_layered_options;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LLabel, LNode, LPort, Layer, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::LabelManagementProcessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions, Origin,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::labels::{
    ILabelManager, LabelManagementOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

struct FixedLabelManager;

impl ILabelManager for FixedLabelManager {
    fn manage_label_size(&self, _label: &dyn Any, target_width: f64) -> Option<KVector> {
        Some(KVector::with_values(target_width / 2.0, 5.0))
    }
}

fn add_label_with_origin(
    target: &mut Vec<Arc<Mutex<LLabel>>>,
    width: f64,
    height: f64,
    id: usize,
) -> Arc<Mutex<LLabel>> {
    let label = Arc::new(Mutex::new(LLabel::with_text(format!("label-{id}"))));
    if let Some(mut label_guard) = label.lock_ok() {
        label_guard.shape().size().x = width;
        label_guard.shape().size().y = height;
        label_guard.set_property(InternalProperties::ORIGIN, Some(Origin::ElkLabel(id)));
    }
    target.push(label.clone());
    label
}

#[test]
fn non_center_mode_manages_node_port_and_edge_labels() {
    init_layered_options();

    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    if let Some(mut graph_guard) = graph.lock_ok() {
        graph_guard.layers_mut().push(layer.clone());
        graph_guard.set_property(
            LabelManagementOptions::LABEL_MANAGER,
            Some(Arc::new(FixedLabelManager) as Arc<dyn ILabelManager>),
        );
        graph_guard.set_property(LayeredOptions::DIRECTION, Some(Direction::Right));
        graph_guard.set_property(LayeredOptions::SPACING_LABEL_LABEL, Some(3.0));
    }

    let node = LNode::new(&graph);
    if let Some(mut node_guard) = node.lock_ok() {
        node_guard.set_node_type(NodeType::Normal);
    }
    LNode::set_layer(&node, Some(layer.clone()));

    let node_label = {
        let mut labels = Vec::new();
        let label = add_label_with_origin(&mut labels, 100.0, 12.0, 1);
        if let Some(mut node_guard) = node.lock_ok() {
            node_guard.labels_mut().extend(labels);
        }
        label
    };

    let node_port = LPort::new();
    if let Some(mut port_guard) = node_port.lock_ok() {
        port_guard.set_side(PortSide::East);
    }
    LPort::set_node(&node_port, Some(node.clone()));
    let port_label = {
        let mut labels = Vec::new();
        let label = add_label_with_origin(&mut labels, 70.0, 11.0, 2);
        if let Some(mut port_guard) = node_port.lock_ok() {
            port_guard.labels_mut().extend(labels);
        }
        label
    };

    let other = LNode::new(&graph);
    if let Some(mut other_guard) = other.lock_ok() {
        other_guard.set_node_type(NodeType::Normal);
    }
    LNode::set_layer(&other, Some(layer.clone()));
    let other_port = LPort::new();
    if let Some(mut port_guard) = other_port.lock_ok() {
        port_guard.set_side(PortSide::West);
    }
    LPort::set_node(&other_port, Some(other));

    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(node_port));
    LEdge::set_target(&edge, Some(other_port));
    let edge_label = {
        let mut labels = Vec::new();
        let label = add_label_with_origin(&mut labels, 90.0, 10.0, 3);
        if let Some(mut edge_guard) = edge.lock_ok() {
            edge_guard.labels_mut().extend(labels);
        }
        label
    };

    let mut processor = LabelManagementProcessor::new(false);
    let mut monitor = NullElkProgressMonitor;
    if let Some(mut graph_guard) = graph.lock_ok() {
        processor.process(&mut graph_guard, &mut monitor);
    }

    let node_size = node_label
        .lock_ok()
        .map(|mut label_guard| *label_guard.shape().size_ref())
        .unwrap_or_default();
    let port_size = port_label
        .lock_ok()
        .map(|mut label_guard| *label_guard.shape().size_ref())
        .unwrap_or_default();
    let edge_size = edge_label
        .lock_ok()
        .map(|mut label_guard| *label_guard.shape().size_ref())
        .unwrap_or_default();

    assert!((node_size.x - 20.0).abs() < 1e-9);
    assert!((node_size.y - 5.0).abs() < 1e-9);
    assert!((port_size.x - 10.0).abs() < 1e-9);
    assert!((port_size.y - 5.0).abs() < 1e-9);
    assert!((edge_size.x - 30.0).abs() < 1e-9);
    assert!((edge_size.y - 5.0).abs() < 1e-9);
}

#[test]
fn center_mode_updates_label_dummy_size_with_spacing_and_edge_thickness() {
    init_layered_options();

    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    if let Some(mut graph_guard) = graph.lock_ok() {
        graph_guard.layers_mut().push(layer.clone());
        graph_guard.set_property(
            LabelManagementOptions::LABEL_MANAGER,
            Some(Arc::new(FixedLabelManager) as Arc<dyn ILabelManager>),
        );
        graph_guard.set_property(LayeredOptions::DIRECTION, Some(Direction::Right));
        graph_guard.set_property(LayeredOptions::SPACING_EDGE_LABEL, Some(2.0));
        graph_guard.set_property(LayeredOptions::SPACING_LABEL_LABEL, Some(1.0));
    }

    let normal = LNode::new(&graph);
    if let Some(mut node_guard) = normal.lock_ok() {
        node_guard.set_node_type(NodeType::Normal);
        node_guard.shape().size().x = 200.0;
        node_guard.shape().size().y = 30.0;
    }
    LNode::set_layer(&normal, Some(layer.clone()));

    let dummy = LNode::new(&graph);
    if let Some(mut node_guard) = dummy.lock_ok() {
        node_guard.set_node_type(NodeType::Label);
        node_guard.shape().size().x = 10.0;
        node_guard.shape().size().y = 10.0;
    }
    LNode::set_layer(&dummy, Some(layer.clone()));

    let source_port = LPort::new();
    if let Some(mut port_guard) = source_port.lock_ok() {
        port_guard.set_side(PortSide::East);
    }
    LPort::set_node(&source_port, Some(normal));

    let dummy_port = LPort::new();
    if let Some(mut port_guard) = dummy_port.lock_ok() {
        port_guard.set_side(PortSide::West);
    }
    LPort::set_node(&dummy_port, Some(dummy.clone()));

    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source_port));
    LEdge::set_target(&edge, Some(dummy_port));
    if let Some(mut edge_guard) = edge.lock_ok() {
        edge_guard.set_property(CoreOptions::EDGE_THICKNESS, Some(4.0));
    }

    let represented = {
        let mut labels = Vec::new();
        add_label_with_origin(&mut labels, 80.0, 9.0, 11);
        add_label_with_origin(&mut labels, 70.0, 7.0, 12);
        labels
    };
    if let Some(mut node_guard) = dummy.lock_ok() {
        node_guard.set_property(InternalProperties::REPRESENTED_LABELS, Some(represented));
    }

    let mut processor = LabelManagementProcessor::new(true);
    let mut monitor = NullElkProgressMonitor;
    if let Some(mut graph_guard) = graph.lock_ok() {
        processor.process(&mut graph_guard, &mut monitor);
    }

    let dummy_size = dummy
        .lock_ok()
        .map(|mut node_guard| *node_guard.shape().size_ref())
        .unwrap_or_default();

    // targetWidth=max(60, maxNormalWidth=200) => manager width=100, height=5 for each label.
    // required.x=100, required.y=5 + 1 + 5 = 11, then + edgeThickness(4) + edgeLabelSpacing(2) = 17.
    assert!((dummy_size.x - 100.0).abs() < 1e-9);
    assert!((dummy_size.y - 17.0).abs() < 1e-9);
}

#[test]
fn vertical_layout_swaps_label_dimensions_from_manager_output() {
    init_layered_options();

    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    if let Some(mut graph_guard) = graph.lock_ok() {
        graph_guard.layers_mut().push(layer.clone());
        graph_guard.set_property(
            LabelManagementOptions::LABEL_MANAGER,
            Some(Arc::new(FixedLabelManager) as Arc<dyn ILabelManager>),
        );
        graph_guard.set_property(LayeredOptions::DIRECTION, Some(Direction::Down));
    }

    let node = LNode::new(&graph);
    if let Some(mut node_guard) = node.lock_ok() {
        node_guard.set_node_type(NodeType::Normal);
    }
    LNode::set_layer(&node, Some(layer));

    let label = {
        let mut labels = Vec::new();
        let label = add_label_with_origin(&mut labels, 55.0, 11.0, 21);
        if let Some(mut node_guard) = node.lock_ok() {
            node_guard.labels_mut().extend(labels);
        }
        label
    };

    let mut processor = LabelManagementProcessor::new(false);
    let mut monitor = NullElkProgressMonitor;
    if let Some(mut graph_guard) = graph.lock_ok() {
        processor.process(&mut graph_guard, &mut monitor);
    }

    let size = label
        .lock_ok()
        .map(|mut label_guard| *label_guard.shape().size_ref())
        .unwrap_or_default();
    // Manager returns (20,5) for node labels; vertical layout swaps to (5,20).
    assert!((size.x - 5.0).abs() < 1e-9);
    assert!((size.y - 20.0).abs() < 1e-9);
}
