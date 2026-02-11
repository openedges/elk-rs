use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LNode, LPort, Layer, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::LabelDummySwitcher;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn add_node(graph: &Arc<std::sync::Mutex<LGraph>>, layer: &Arc<std::sync::Mutex<Layer>>, node_type: NodeType) -> Arc<std::sync::Mutex<LNode>> {
    let node = LNode::new(graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.set_node_type(node_type);
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port(node: &Arc<std::sync::Mutex<LNode>>, side: PortSide) -> Arc<std::sync::Mutex<LPort>> {
    let port = LPort::new();
    {
        let mut port_guard = port.lock().expect("port lock");
        port_guard.set_side(side);
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn connect(source: &Arc<std::sync::Mutex<LPort>>, target: &Arc<std::sync::Mutex<LPort>>) -> Arc<std::sync::Mutex<LEdge>> {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
    edge
}

fn init_layered_metadata() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

#[test]
fn moves_label_dummy_to_median_layer() {
    init_layered_metadata();
    let graph = LGraph::new();
    let layer0 = Layer::new(&graph);
    let layer1 = Layer::new(&graph);
    let layer2 = Layer::new(&graph);
    {
        let mut graph_guard = graph.lock().expect("graph lock");
        graph_guard.layers_mut().push(layer0.clone());
        graph_guard.layers_mut().push(layer1.clone());
        graph_guard.layers_mut().push(layer2.clone());
        graph_guard.set_property(
            LayeredOptions::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY,
            None,
        );
    }

    let source = add_node(&graph, &layer0, NodeType::Normal);
    let label_dummy = add_node(&graph, &layer0, NodeType::Label);
    let long_edge1 = add_node(&graph, &layer1, NodeType::LongEdge);
    let long_edge2 = add_node(&graph, &layer2, NodeType::LongEdge);
    let target = add_node(&graph, &layer2, NodeType::Normal);

    let source_out = add_port(&source, PortSide::East);
    let label_in = add_port(&label_dummy, PortSide::West);
    let label_out = add_port(&label_dummy, PortSide::East);
    let long1_in = add_port(&long_edge1, PortSide::West);
    let long1_out = add_port(&long_edge1, PortSide::East);
    let long2_in = add_port(&long_edge2, PortSide::West);
    let long2_out = add_port(&long_edge2, PortSide::East);
    let target_in = add_port(&target, PortSide::West);

    connect(&source_out, &label_in);
    connect(&label_out, &long1_in);
    connect(&long1_out, &long2_in);
    connect(&long2_out, &target_in);

    let mut processor = LabelDummySwitcher::default();
    let mut monitor = NullElkProgressMonitor;
    processor.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let layer0_nodes = layer0.lock().expect("layer0 lock").nodes().clone();
    let layer1_nodes = layer1.lock().expect("layer1 lock").nodes().clone();

    assert!(
        layer0_nodes.iter().any(|node| Arc::ptr_eq(node, &long_edge1)),
        "long edge dummy should swap into leftmost layer"
    );
    assert!(
        layer1_nodes.iter().any(|node| Arc::ptr_eq(node, &label_dummy)),
        "label dummy should move to median layer"
    );
}
