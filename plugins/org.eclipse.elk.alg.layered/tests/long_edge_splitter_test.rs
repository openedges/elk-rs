use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::LongEdgeSplitter;
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

fn layer_index(layers: &[LayerRef], target_layer: &LayerRef) -> usize {
    layers
        .iter()
        .position(|layer| Arc::ptr_eq(layer, target_layer))
        .unwrap_or(0)
}

#[test]
fn long_edge_splitter_makes_edges_connect_adjacent_layers() {
    let (graph, layers) = layered_graph_with_three_layers();
    let source = add_node(&graph, &layers[0]);
    let target = add_node(&graph, &layers[2]);
    let source_port = add_port(&source, PortSide::East);
    let target_port = add_port(&target, PortSide::West);
    connect(&source_port, &target_port);

    let mut processor = LongEdgeSplitter;
    let mut monitor = NullElkProgressMonitor;
    processor.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    for (i, layer) in layers.iter().enumerate() {
        let nodes = layer.lock().expect("layer lock").nodes().clone();
        for node in nodes {
            let outgoing = node
                .lock()
                .expect("node lock")
                .outgoing_edges();
            for edge in outgoing {
                let target_layer = edge
                    .lock()
                    .expect("edge lock")
                    .target()
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
                    .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.layer()))
                    .expect("target layer");
                let t_idx = layer_index(&layers, &target_layer);
                assert!(t_idx > i, "edge must point forward");
                assert_eq!(t_idx, i + 1, "edge must connect adjacent layers");
            }
        }
    }
}

#[test]
fn long_edge_splitter_inserts_long_edge_dummy_nodes() {
    let (graph, layers) = layered_graph_with_three_layers();
    let source = add_node(&graph, &layers[0]);
    let target = add_node(&graph, &layers[2]);
    let source_port = add_port(&source, PortSide::East);
    let target_port = add_port(&target, PortSide::West);
    connect(&source_port, &target_port);

    let mut processor = LongEdgeSplitter;
    let mut monitor = NullElkProgressMonitor;
    processor.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let middle_nodes = layers[1].lock().expect("middle layer lock").nodes().clone();
    assert!(
        middle_nodes
            .iter()
            .any(|node| node.lock().ok().map(|n| n.node_type()) == Some(NodeType::LongEdge)),
        "middle layer should contain a long edge dummy",
    );
}
