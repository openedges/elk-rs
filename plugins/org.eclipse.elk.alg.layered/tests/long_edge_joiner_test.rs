use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::{
    LongEdgeJoiner, LongEdgeSplitter,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn layered_graph_with_three_layers() -> (LGraphRef, Vec<LayerRef>) {
    let graph = LGraph::new();
    let mut layers = Vec::new();
    {
        let mut graph_guard = graph.lock().expect("graph lock");
        for _ in 0..3 {
            let layer = Layer::new(&graph);
            graph_guard.layers_mut().push(layer.clone());
            layers.push(layer);
        }
    }
    (graph, layers)
}

fn add_node(graph: &LGraphRef, layer: &LayerRef) -> LNodeRef {
    let node = LNode::new(graph);
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port(node: &LNodeRef, side: PortSide) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LPortRef {
    let port = LPort::new();
    port.lock().expect("port lock").set_side(side);
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn connect(
    source: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LPortRef,
    target: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LPortRef,
) {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
}

#[test]
fn long_edge_joiner_removes_long_edge_dummy_nodes() {
    let (graph, layers) = layered_graph_with_three_layers();
    let source = add_node(&graph, &layers[0]);
    let target = add_node(&graph, &layers[2]);
    let source_port = add_port(&source, PortSide::East);
    let target_port = add_port(&target, PortSide::West);
    connect(&source_port, &target_port);

    let mut splitter = LongEdgeSplitter;
    let mut joiner = LongEdgeJoiner;
    let mut monitor = NullElkProgressMonitor;
    splitter.process(&mut graph.lock().expect("graph lock"), &mut monitor);
    joiner.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    for layer in &layers {
        let nodes = layer.lock().expect("layer lock").nodes().clone();
        for node in nodes {
            let ty = node.lock().expect("node lock").node_type();
            assert_ne!(ty, NodeType::LongEdge);
        }
    }
}
