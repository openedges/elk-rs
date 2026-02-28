use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LLabel, LNode, LPort, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::LabelDummyInserter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

#[test]
fn inserts_label_dummy_and_moves_center_labels() {
    let graph = LGraph::new();
    {
        let mut graph_guard = graph.lock().expect("graph lock");
        graph_guard.set_property(LayeredOptions::DIRECTION, Some(Direction::Right));
        graph_guard.set_property(LayeredOptions::SPACING_EDGE_LABEL, Some(2.0));
        graph_guard.set_property(LayeredOptions::SPACING_LABEL_LABEL, Some(1.0));
    }

    let source = LNode::new(&graph);
    let target = LNode::new(&graph);
    {
        let mut graph_guard = graph.lock().expect("graph lock");
        graph_guard.layerless_nodes_mut().push(source.clone());
        graph_guard.layerless_nodes_mut().push(target.clone());
    }

    let source_port = LPort::new();
    {
        let mut port_guard = source_port.lock().expect("source port lock");
        port_guard.set_side(PortSide::East);
    }
    LPort::set_node(&source_port, Some(source.clone()));

    let target_port = LPort::new();
    {
        let mut port_guard = target_port.lock().expect("target port lock");
        port_guard.set_side(PortSide::West);
    }
    LPort::set_node(&target_port, Some(target.clone()));

    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source_port));
    LEdge::set_target(&edge, Some(target_port));
    {
        let mut edge_guard = edge.lock().expect("edge lock");
        edge_guard.set_property(CoreOptions::EDGE_THICKNESS, Some(2.0));
    }

    let label = Arc::new(Mutex::new(LLabel::with_text("center")));
    {
        let mut label_guard = label.lock().expect("label lock");
        label_guard.shape().size().x = 10.0;
        label_guard.shape().size().y = 5.0;
        label_guard.set_property(
            LayeredOptions::EDGE_LABELS_PLACEMENT,
            Some(EdgeLabelPlacement::Center),
        );
    }
    {
        let mut edge_guard = edge.lock().expect("edge lock");
        edge_guard.labels_mut().push(label.clone());
    }

    let mut processor = LabelDummyInserter;
    let mut monitor = NullElkProgressMonitor;
    processor.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let layerless_nodes = graph.lock().expect("graph lock").layerless_nodes().clone();
    assert_eq!(layerless_nodes.len(), 3, "label dummy should be added");

    let dummy = layerless_nodes
        .iter()
        .find(|node| {
            node.lock()
                .ok()
                .map(|node_guard| node_guard.node_type() == NodeType::Label)
                .unwrap_or(false)
        })
        .expect("label dummy node");

    let represented = {
        let mut dummy_guard = dummy.lock().expect("dummy lock");
        dummy_guard
            .get_property(InternalProperties::REPRESENTED_LABELS)
            .unwrap_or_default()
    };
    assert_eq!(represented.len(), 1, "center label must move to dummy");

    let dummy_size = {
        let mut dummy_guard = dummy.lock().expect("dummy lock");
        *dummy_guard.shape().size_ref()
    };
    assert_eq!(dummy_size.x, 10.0);
    assert_eq!(dummy_size.y, 11.0);

    let edge_labels = edge.lock().expect("edge lock").labels().clone();
    assert!(
        edge_labels.is_empty(),
        "center label must be removed from edge"
    );
}
