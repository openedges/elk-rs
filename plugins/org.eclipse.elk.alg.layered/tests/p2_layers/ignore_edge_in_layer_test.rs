use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LNode, LPort,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p2layers::network_simplex_layerer::NetworkSimplexLayerer;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;

fn init() {
    LayoutMetaDataService::get_instance()
        .register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

/// Build a simple A→B graph. If `ignore` is true, set ignoreEdgeInLayer on the edge.
fn build_two_node_graph(ignore: bool) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef {
    let graph = LGraph::new();
    let node_a = LNode::new(&graph);
    let node_b = LNode::new(&graph);

    let port_a = LPort::new();
    LPort::set_node(&port_a, Some(node_a.clone()));

    let port_b = LPort::new();
    LPort::set_node(&port_b, Some(node_b.clone()));

    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(port_a));
    LEdge::set_target(&edge, Some(port_b));
    if ignore {
        edge.lock()
            .expect("edge lock")
            .set_property(LayeredOptions::LAYERING_IGNORE_EDGE_IN_LAYER, Some(true));
    }

    {
        let mut g = graph.lock().expect("graph lock");
        g.layerless_nodes_mut().push(node_a);
        g.layerless_nodes_mut().push(node_b);
    }
    graph
}

#[test]
fn ignore_edge_in_layer_places_nodes_in_same_layer() {
    init();
    let graph = build_two_node_graph(true);
    {
        let mut g = graph.lock().expect("graph lock");
        let mut layerer = NetworkSimplexLayerer::new();
        let mut monitor = BasicProgressMonitor::new();
        layerer.process(&mut g, &mut monitor);
    }

    let g = graph.lock().expect("graph lock");
    assert!(g.layerless_nodes().is_empty(), "no layerless nodes remain");

    // Both nodes should be in the same layer (delta=0 allows this)
    let layers = g.layers();
    assert_eq!(layers.len(), 1, "both nodes should be in a single layer");
    let layer = layers[0].lock().expect("layer lock");
    assert_eq!(layer.nodes().len(), 2);
}

#[test]
fn normal_edge_places_nodes_in_different_layers() {
    init();
    let graph = build_two_node_graph(false);
    {
        let mut g = graph.lock().expect("graph lock");
        let mut layerer = NetworkSimplexLayerer::new();
        let mut monitor = BasicProgressMonitor::new();
        layerer.process(&mut g, &mut monitor);
    }

    let g = graph.lock().expect("graph lock");
    assert!(g.layerless_nodes().is_empty());

    // Normal edge: nodes should be in different layers
    let layers = g.layers();
    assert!(layers.len() >= 2, "nodes should be in separate layers");
}

#[test]
fn ignore_edge_in_layer_reverses_same_layer_east_west_edge() {
    init();
    let graph = LGraph::new();
    let node_a = LNode::new(&graph);
    let node_b = LNode::new(&graph);

    let port_a = LPort::new();
    {
        let mut pg = port_a.lock().expect("port lock");
        pg.set_side(org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide::East);
    }
    LPort::set_node(&port_a, Some(node_a.clone()));

    let port_b = LPort::new();
    {
        let mut pg = port_b.lock().expect("port lock");
        pg.set_side(org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide::West);
    }
    LPort::set_node(&port_b, Some(node_b.clone()));

    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(port_a));
    LEdge::set_target(&edge, Some(port_b));
    edge.lock()
        .expect("edge lock")
        .set_property(LayeredOptions::LAYERING_IGNORE_EDGE_IN_LAYER, Some(true));

    {
        let mut g = graph.lock().expect("graph lock");
        g.layerless_nodes_mut().push(node_a);
        g.layerless_nodes_mut().push(node_b);
    }

    {
        let mut g = graph.lock().expect("graph lock");
        let mut layerer = NetworkSimplexLayerer::new();
        let mut monitor = BasicProgressMonitor::new();
        layerer.process(&mut g, &mut monitor);
    }

    // Edge should be reversed (same-layer EAST→WEST with ignoreEdgeInLayer)
    let reversed = edge
        .lock()
        .expect("edge lock")
        .get_property(InternalProperties::REVERSED)
        .unwrap_or(false);
    assert!(reversed, "same-layer EAST→WEST ignoreEdgeInLayer edge should be reversed");
}
