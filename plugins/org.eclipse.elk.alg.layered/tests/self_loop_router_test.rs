use std::sync::{Arc, Mutex};

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LLabel, LNode, LNodeRef, LPort, LPortRef, Layer,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::{
    SelfLoopPortRestorer, SelfLoopPostProcessor, SelfLoopPreProcessor, SelfLoopRouter,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::loops::SelfLoopType;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredMetaDataProvider, LayeredOptions, SelfLoopDistributionStrategy,
    SelfLoopOrderingStrategy,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::internal_properties::Origin;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn create_port(node: &LNodeRef, side: PortSide, x: f64, y: f64) -> LPortRef {
    let port = LPort::new();
    {
        let mut port_guard = port.lock().expect("port lock");
        port_guard.set_side(side);
        port_guard.shape().position().x = x;
        port_guard.shape().position().y = y;
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn add_edge(source: &LPortRef, target: &LPortRef) -> LEdgeRef {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
    edge
}

fn add_single_label(edge: &LEdgeRef, width: f64, height: f64) {
    add_single_label_with_inline(edge, width, height, false);
}

fn add_single_label_with_inline(edge: &LEdgeRef, width: f64, height: f64, inline: bool) {
    let label = Arc::new(Mutex::new(LLabel::with_text("self-loop")));
    {
        let mut label_guard = label.lock().expect("label lock");
        label_guard.shape().size().x = width;
        label_guard.shape().size().y = height;
        label_guard.set_property(LayeredOptions::EDGE_LABELS_INLINE, Some(inline));
    }
    edge.lock().expect("edge lock").labels_mut().push(label);
}

fn run_processor(processor: &mut dyn ILayoutProcessor<LGraph>, graph: &LGraphRef) {
    let mut monitor = NullElkProgressMonitor;
    processor.process(&mut graph.lock().expect("graph lock"), &mut monitor);
}

fn init_layered_metadata() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

type CornerDummyCombinedGraph = (
    LGraphRef,
    LNodeRef,
    LPortRef,
    LPortRef,
    LPortRef,
    Vec<LPortRef>,
    Vec<LPortRef>,
    Vec<LEdgeRef>,
);

fn build_self_loop_graph() -> (LGraphRef, LNodeRef, Vec<LEdgeRef>) {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.shape().size().x = 60.0;
        node_guard.shape().size().y = 50.0;
        node_guard.shape().position().x = 15.0;
        node_guard.shape().position().y = 20.0;
        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::Free));
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(node.clone());

    let east = create_port(&node, PortSide::East, 60.0, 14.0);
    let west = create_port(&node, PortSide::West, 0.0, 32.0);
    let north = create_port(&node, PortSide::North, 24.0, 0.0);

    let first = add_edge(&east, &west);
    let second = add_edge(&north, &east);

    (graph, node, vec![first, second])
}

fn build_parallel_north_self_loop_graph() -> (LGraphRef, LNodeRef, Vec<LEdgeRef>) {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.shape().size().x = 80.0;
        node_guard.shape().size().y = 60.0;
        node_guard.shape().position().x = 10.0;
        node_guard.shape().position().y = 30.0;
        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::Free));
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(node.clone());

    let north_0 = create_port(&node, PortSide::North, 10.0, 0.0);
    let north_1 = create_port(&node, PortSide::North, 30.0, 0.0);
    let north_2 = create_port(&node, PortSide::North, 50.0, 0.0);
    let north_3 = create_port(&node, PortSide::North, 70.0, 0.0);

    let first = add_edge(&north_0, &north_1);
    let second = add_edge(&north_2, &north_3);

    (graph, node, vec![first, second])
}

fn build_parallel_north_self_loop_graph_with_labels() -> (LGraphRef, LNodeRef, Vec<LEdgeRef>) {
    let (graph, node, edges) = build_parallel_north_self_loop_graph();
    for edge in &edges {
        add_single_label(edge, 14.0, 8.0);
    }
    (graph, node, edges)
}

fn build_parallel_north_self_loop_graph_with_labels_and_sequenced_ordering() -> (LGraphRef, LNodeRef, Vec<LEdgeRef>) {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.shape().size().x = 80.0;
        node_guard.shape().size().y = 60.0;
        node_guard.shape().position().x = 10.0;
        node_guard.shape().position().y = 30.0;
        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::Free));
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
        node_guard.set_property(
            LayeredOptions::EDGE_ROUTING_SELF_LOOP_ORDERING,
            Some(SelfLoopOrderingStrategy::Sequenced),
        );
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(node.clone());

    let north_0 = create_port(&node, PortSide::North, 8.0, 0.0);
    let north_1 = create_port(&node, PortSide::North, 20.0, 0.0);
    let north_2 = create_port(&node, PortSide::North, 58.0, 0.0);
    let north_3 = create_port(&node, PortSide::North, 72.0, 0.0);

    let first = add_edge(&north_0, &north_1);
    let second = add_edge(&north_2, &north_3);
    add_single_label(&first, 14.0, 8.0);
    add_single_label(&second, 14.0, 8.0);

    (graph, node, vec![first, second])
}

fn build_north_south_self_loop_graph_with_label(inline: bool) -> (LGraphRef, LNodeRef, LEdgeRef) {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.shape().size().x = 80.0;
        node_guard.shape().size().y = 60.0;
        node_guard.shape().position().x = 10.0;
        node_guard.shape().position().y = 20.0;
        // Keep original north/south side assignment so the loop remains two-sided-opposing.
        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedOrder));
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(node.clone());

    let north = create_port(&node, PortSide::North, 40.0, 0.0);
    let south = create_port(&node, PortSide::South, 40.0, 60.0);
    let edge = add_edge(&north, &south);
    add_single_label_with_inline(&edge, 20.0, 10.0, inline);
    (graph, node, edge)
}

fn build_opposing_self_loop_with_side_penalty(
    east_connected: bool,
    west_connected: bool,
) -> (LGraphRef, LNodeRef, LEdgeRef) {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.shape().size().x = 80.0;
        node_guard.shape().size().y = 60.0;
        node_guard.shape().position().x = 10.0;
        node_guard.shape().position().y = 20.0;
        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedOrder));
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(node.clone());

    let north = create_port(&node, PortSide::North, 40.0, 0.0);
    let east = create_port(&node, PortSide::East, 80.0, 30.0);
    let south = create_port(&node, PortSide::South, 40.0, 60.0);
    let west = create_port(&node, PortSide::West, 0.0, 30.0);
    let self_loop = add_edge(&north, &south);

    let external = LNode::new(&graph);
    {
        let mut external_guard = external.lock().expect("external node lock");
        external_guard.shape().size().x = 20.0;
        external_guard.shape().size().y = 20.0;
        external_guard.shape().position().x = 140.0;
        external_guard.shape().position().y = 20.0;
        external_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedOrder));
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(external.clone());
    let ext_west = create_port(&external, PortSide::West, 0.0, 10.0);
    let ext_east = create_port(&external, PortSide::East, 20.0, 10.0);

    if east_connected {
        let _ = add_edge(&east, &ext_west);
    }
    if west_connected {
        let _ = add_edge(&west, &ext_east);
    }

    (graph, node, self_loop)
}

fn build_north_west_corner_mixed_graph() -> (
    LGraphRef,
    LNodeRef,
    LPortRef,
    LPortRef,
    LPortRef,
    LPortRef,
) {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.shape().size().x = 90.0;
        node_guard.shape().size().y = 60.0;
        node_guard.shape().position().x = 15.0;
        node_guard.shape().position().y = 20.0;
        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedSide));
        node_guard.set_property(
            InternalProperties::ORIGINAL_PORT_CONSTRAINTS,
            Some(PortConstraints::FixedSide),
        );
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(node.clone());

    let hidden_north = create_port(&node, PortSide::North, 30.0, 0.0);
    let hidden_west = create_port(&node, PortSide::West, 0.0, 30.0);
    let visible_north = create_port(&node, PortSide::North, 60.0, 0.0);
    let visible_west = create_port(&node, PortSide::West, 0.0, 45.0);

    let _self_loop = add_edge(&hidden_west, &hidden_north);

    let external = LNode::new(&graph);
    {
        let mut ext_guard = external.lock().expect("external node lock");
        ext_guard.shape().size().x = 20.0;
        ext_guard.shape().size().y = 20.0;
        ext_guard.shape().position().x = 130.0;
        ext_guard.shape().position().y = 20.0;
        ext_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedOrder));
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(external.clone());
    let ext_west = create_port(&external, PortSide::West, 0.0, 10.0);
    let ext_east = create_port(&external, PortSide::East, 20.0, 10.0);
    let _ = add_edge(&visible_west, &ext_west);
    let _ = add_edge(&ext_east, &visible_north);

    (
        graph,
        node,
        hidden_north,
        hidden_west,
        visible_north,
        visible_west,
    )
}

fn add_chain_self_loop_ports(
    node: &LNodeRef,
    side: PortSide,
    count: usize,
    start_x: f64,
) -> Vec<LPortRef> {
    let mut ports = Vec::with_capacity(count);
    for index in 0..count {
        let x = start_x + (index as f64) * 7.0;
        let y = 0.0;
        ports.push(create_port(node, side, x, y));
    }
    for index in 1..ports.len() {
        let _ = add_edge(&ports[index - 1], &ports[index]);
    }
    ports
}

fn build_equally_distribution_graph_with_unique_loop_sizes(
) -> (LGraphRef, LNodeRef, LPortRef, LPortRef) {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.shape().size().x = 120.0;
        node_guard.shape().size().y = 70.0;
        node_guard.shape().position().x = 15.0;
        node_guard.shape().position().y = 20.0;
        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::Free));
        node_guard.set_property(
            LayeredOptions::EDGE_ROUTING_SELF_LOOP_DISTRIBUTION,
            Some(SelfLoopDistributionStrategy::Equally),
        );
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(node.clone());

    let _loop_6 = add_chain_self_loop_ports(&node, PortSide::North, 6, 4.0);
    let _loop_5 = add_chain_self_loop_ports(&node, PortSide::North, 5, 50.0);
    let _loop_4 = add_chain_self_loop_ports(&node, PortSide::North, 4, 90.0);
    let _loop_3 = add_chain_self_loop_ports(&node, PortSide::North, 3, 130.0);
    let loop_2 = add_chain_self_loop_ports(&node, PortSide::North, 2, 160.0);

    (graph, node, loop_2[0].clone(), loop_2[1].clone())
}

fn build_south_middle_with_east_dummy_graph() -> (LGraphRef, LNodeRef, LPortRef, Vec<LPortRef>) {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.shape().size().x = 120.0;
        node_guard.shape().size().y = 70.0;
        node_guard.shape().position().x = 15.0;
        node_guard.shape().position().y = 20.0;
        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedSide));
        node_guard.set_property(
            InternalProperties::ORIGINAL_PORT_CONSTRAINTS,
            Some(PortConstraints::FixedSide),
        );
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(node.clone());

    let hidden0 = create_port(&node, PortSide::South, 30.0, 70.0);
    let hidden1 = create_port(&node, PortSide::South, 50.0, 70.0);
    let _ = add_edge(&hidden0, &hidden1);

    let visible_south = create_port(&node, PortSide::South, 80.0, 70.0);
    let external = LNode::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(external.clone());
    let ext_west = create_port(&external, PortSide::West, 0.0, 10.0);
    let _ = add_edge(&visible_south, &ext_west);

    // Simulate a north/south dummy that records an EAST connection for the visible south port.
    let port_dummy = LNode::new(&graph);
    let dummy_port = create_port(&port_dummy, PortSide::East, 0.0, 0.0);
    if let Ok(mut dummy_port_guard) = dummy_port.lock() {
        dummy_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(visible_south.clone())),
        );
    }
    let dummy_external = LNode::new(&graph);
    let dummy_target = create_port(&dummy_external, PortSide::West, 0.0, 0.0);
    let _ = add_edge(&dummy_port, &dummy_target);
    if let Ok(mut visible_guard) = visible_south.lock() {
        visible_guard.set_property(InternalProperties::PORT_DUMMY, Some(port_dummy));
    }

    (graph, node, visible_south, vec![hidden0, hidden1])
}

fn build_north_middle_with_west_dummy_graph() -> (LGraphRef, LNodeRef, LPortRef, Vec<LPortRef>) {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.shape().size().x = 120.0;
        node_guard.shape().size().y = 70.0;
        node_guard.shape().position().x = 15.0;
        node_guard.shape().position().y = 20.0;
        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedSide));
        node_guard.set_property(
            InternalProperties::ORIGINAL_PORT_CONSTRAINTS,
            Some(PortConstraints::FixedSide),
        );
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(node.clone());

    let hidden0 = create_port(&node, PortSide::North, 30.0, 0.0);
    let hidden1 = create_port(&node, PortSide::North, 50.0, 0.0);
    let _ = add_edge(&hidden0, &hidden1);

    let visible_north = create_port(&node, PortSide::North, 80.0, 0.0);
    let external = LNode::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(external.clone());
    let ext_east = create_port(&external, PortSide::East, 10.0, 10.0);
    let _ = add_edge(&ext_east, &visible_north);

    // Simulate a north/south dummy that records a WEST connection for the visible north port.
    let port_dummy = LNode::new(&graph);
    let dummy_port = create_port(&port_dummy, PortSide::West, 0.0, 0.0);
    if let Ok(mut dummy_port_guard) = dummy_port.lock() {
        dummy_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(visible_north.clone())),
        );
    }
    let dummy_external = LNode::new(&graph);
    let dummy_source = create_port(&dummy_external, PortSide::East, 0.0, 0.0);
    let _ = add_edge(&dummy_source, &dummy_port);
    if let Ok(mut visible_guard) = visible_north.lock() {
        visible_guard.set_property(InternalProperties::PORT_DUMMY, Some(port_dummy));
    }

    (graph, node, visible_north, vec![hidden0, hidden1])
}

fn build_corner_and_dummy_combined_graph() -> CornerDummyCombinedGraph {
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.shape().size().x = 150.0;
        node_guard.shape().size().y = 90.0;
        node_guard.shape().position().x = 15.0;
        node_guard.shape().position().y = 20.0;
        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedSide));
        node_guard.set_property(
            InternalProperties::ORIGINAL_PORT_CONSTRAINTS,
            Some(PortConstraints::FixedSide),
        );
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(node.clone());

    let hidden_west_corner = create_port(&node, PortSide::West, 0.0, 20.0);
    let hidden_north_corner = create_port(&node, PortSide::North, 22.0, 0.0);
    let corner_loop = add_edge(&hidden_west_corner, &hidden_north_corner);
    add_single_label(&corner_loop, 12.0, 6.0);

    let hidden_north_mid0 = create_port(&node, PortSide::North, 66.0, 0.0);
    let hidden_north_mid1 = create_port(&node, PortSide::North, 86.0, 0.0);
    let north_mid_loop = add_edge(&hidden_north_mid0, &hidden_north_mid1);
    add_single_label(&north_mid_loop, 11.0, 6.0);

    let hidden_south_mid0 = create_port(&node, PortSide::South, 70.0, 90.0);
    let hidden_south_mid1 = create_port(&node, PortSide::South, 90.0, 90.0);
    let south_mid_loop = add_edge(&hidden_south_mid0, &hidden_south_mid1);
    add_single_label(&south_mid_loop, 11.0, 6.0);

    let visible_north = create_port(&node, PortSide::North, 124.0, 0.0);
    let ext_north = LNode::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(ext_north.clone());
    let ext_north_east = create_port(&ext_north, PortSide::East, 10.0, 10.0);
    let _ = add_edge(&ext_north_east, &visible_north);

    let north_dummy = LNode::new(&graph);
    let north_dummy_port = create_port(&north_dummy, PortSide::West, 0.0, 0.0);
    if let Ok(mut dummy_port_guard) = north_dummy_port.lock() {
        dummy_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(visible_north.clone())),
        );
    }
    let north_dummy_external = LNode::new(&graph);
    let north_dummy_source = create_port(&north_dummy_external, PortSide::East, 0.0, 0.0);
    let _ = add_edge(&north_dummy_source, &north_dummy_port);
    if let Ok(mut visible_guard) = visible_north.lock() {
        visible_guard.set_property(InternalProperties::PORT_DUMMY, Some(north_dummy));
    }

    let visible_south = create_port(&node, PortSide::South, 124.0, 90.0);
    let ext_south = LNode::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(ext_south.clone());
    let ext_south_west = create_port(&ext_south, PortSide::West, 0.0, 10.0);
    let _ = add_edge(&visible_south, &ext_south_west);

    let south_dummy = LNode::new(&graph);
    let south_dummy_port = create_port(&south_dummy, PortSide::East, 0.0, 0.0);
    if let Ok(mut dummy_port_guard) = south_dummy_port.lock() {
        dummy_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(visible_south.clone())),
        );
    }
    let south_dummy_external = LNode::new(&graph);
    let south_dummy_target = create_port(&south_dummy_external, PortSide::West, 0.0, 0.0);
    let _ = add_edge(&south_dummy_port, &south_dummy_target);
    if let Ok(mut visible_guard) = visible_south.lock() {
        visible_guard.set_property(InternalProperties::PORT_DUMMY, Some(south_dummy));
    }

    (
        graph,
        node,
        visible_north,
        visible_south,
        hidden_north_corner,
        vec![hidden_north_mid0, hidden_north_mid1],
        vec![hidden_south_mid0, hidden_south_mid1],
        vec![corner_loop, north_mid_loop, south_mid_loop],
    )
}

#[test]
fn self_loop_router_keeps_duplicate_bend_points_for_same_port() {
    init_layered_metadata();
    let graph = LGraph::new();
    let node = LNode::new(&graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.shape().size().x = 50.0;
        node_guard.shape().size().y = 40.0;
        node_guard.shape().position().x = 10.0;
        node_guard.shape().position().y = 20.0;
        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::Free));
        node_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(node.clone());

    let port = create_port(&node, PortSide::North, 20.0, 0.0);
    let edge = add_edge(&port, &port);

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    let layer = Layer::new(&graph);
    {
        graph.lock()
            .expect("graph lock")
            .layers_mut()
            .push(layer.clone());
    }
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);
    let mut router = SelfLoopRouter;
    run_processor(&mut router, &graph);
    let mut post = SelfLoopPostProcessor;
    run_processor(&mut post, &graph);

    let bends = edge
        .lock()
        .expect("edge lock")
        .bend_points_ref()
        .to_array();
    assert_eq!(bends.len(), 2, "duplicate outer bend points should be preserved");
    assert!(
        (bends[0].x - bends[1].x).abs() < 1e-9 && (bends[0].y - bends[1].y).abs() < 1e-9,
        "expected identical bend points, got {bends:?}"
    );
}

#[test]
fn self_loop_router_creates_outside_bend_points_for_hidden_loops() {
    init_layered_metadata();
    let (graph, node, edges) = build_self_loop_graph();

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    let layer = Layer::new(&graph);
    {
        graph.lock()
            .expect("graph lock")
            .layers_mut()
            .push(layer.clone());
    }
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);
    let mut router = SelfLoopRouter;
    run_processor(&mut router, &graph);
    let mut post = SelfLoopPostProcessor;
    run_processor(&mut post, &graph);

    let (node_x, node_y, node_w, node_h) = node
        .lock()
        .ok()
        .map(|mut node_guard| {
            (
                node_guard.shape().position_ref().x,
                node_guard.shape().position_ref().y,
                node_guard.shape().size_ref().x,
                node_guard.shape().size_ref().y,
            )
        })
        .unwrap_or((0.0, 0.0, 0.0, 0.0));

    for edge in edges {
        let edge_guard = edge.lock().expect("edge lock");
        assert!(edge_guard.source().is_some());
        assert!(edge_guard.target().is_some());
        assert!(
            !edge_guard.bend_points_ref().is_empty(),
            "self-loop edges need bend points after routing"
        );

        let has_outside_point = edge_guard
            .bend_points_ref()
            .iter()
            .any(|point| {
                point.x < node_x
                    || point.x > node_x + node_w
                    || point.y < node_y
                    || point.y > node_y + node_h
            });
        assert!(has_outside_point, "self-loop route should leave node bounds");
    }
}

#[test]
fn self_loop_router_assigns_separate_slots_for_parallel_north_loops() {
    init_layered_metadata();
    let (graph, node, edges) = build_parallel_north_self_loop_graph();

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    let layer = Layer::new(&graph);
    {
        graph.lock()
            .expect("graph lock")
            .layers_mut()
            .push(layer.clone());
    }
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);
    let mut router = SelfLoopRouter;
    run_processor(&mut router, &graph);
    let mut post = SelfLoopPostProcessor;
    run_processor(&mut post, &graph);

    let node_y = node
        .lock()
        .expect("node lock")
        .shape()
        .position_ref()
        .y;
    for edge in edges {
        let edge_guard = edge.lock().expect("edge lock");
        assert!(
            !edge_guard.bend_points_ref().is_empty(),
            "self-loop edges need bend points after routing"
        );
        let min_y = edge_guard
            .bend_points_ref()
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
        assert!(min_y < node_y, "north self-loop should route above node");
    }
}

#[test]
fn self_loop_router_places_labels_above_parallel_north_loops() {
    init_layered_metadata();
    let (graph, node, edges) = build_parallel_north_self_loop_graph_with_labels();

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    let layer = Layer::new(&graph);
    {
        graph.lock()
            .expect("graph lock")
            .layers_mut()
            .push(layer.clone());
    }
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);
    let mut router = SelfLoopRouter;
    run_processor(&mut router, &graph);
    let mut post = SelfLoopPostProcessor;
    run_processor(&mut post, &graph);

    let (node_x, node_y, node_width) = {
        let mut node_guard = node.lock().expect("node lock");
        (
            node_guard.shape().position_ref().x,
            node_guard.shape().position_ref().y,
            node_guard.shape().size_ref().x,
        )
    };

    let mut label_y_positions = Vec::new();
    for edge in edges {
        let label = {
            let edge_guard = edge.lock().expect("edge lock");
            assert_eq!(edge_guard.labels().len(), 1, "expected one label per edge");
            edge_guard.labels()[0].clone()
        };

        let (x, y, w, h) = {
            let mut label_guard = label.lock().expect("label lock");
            (
                label_guard.shape().position_ref().x,
                label_guard.shape().position_ref().y,
                label_guard.shape().size_ref().x,
                label_guard.shape().size_ref().y,
            )
        };

        assert!(x.is_finite() && y.is_finite(), "label position must be finite");
        assert!(w > 0.0 && h > 0.0, "label size must be positive");
        assert!(y + h < node_y, "north self-loop labels should be above node");
        assert!(
            x >= node_x - 1e-6 && x + w <= node_x + node_width + 1e-6,
            "centered north labels should stay within node width"
        );

        label_y_positions.push(y);
    }

    label_y_positions.sort_by(|left, right| left.total_cmp(right));
    assert!(
        (label_y_positions[1] - label_y_positions[0]).abs() > 1e-3,
        "parallel north loops should not share the same label lane"
    );
}

#[test]
fn self_loop_router_sequences_one_sided_north_labels_left_and_right() {
    init_layered_metadata();
    let (graph, node, edges) = build_parallel_north_self_loop_graph_with_labels_and_sequenced_ordering();

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    let layer = Layer::new(&graph);
    {
        graph.lock()
            .expect("graph lock")
            .layers_mut()
            .push(layer.clone());
    }
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);
    let mut router = SelfLoopRouter;
    run_processor(&mut router, &graph);
    let mut post = SelfLoopPostProcessor;
    run_processor(&mut post, &graph);

    let (node_x, node_y, node_width) = {
        let mut node_guard = node.lock().expect("node lock");
        (
            node_guard.shape().position_ref().x,
            node_guard.shape().position_ref().y,
            node_guard.shape().size_ref().x,
        )
    };
    let node_center_x = node_x + node_width / 2.0;

    let mut label_centers = Vec::new();
    for edge in edges {
        let label = {
            let edge_guard = edge.lock().expect("edge lock");
            edge_guard
                .labels()
                .first()
                .expect("expected one label per edge")
                .clone()
        };

        let (x, y, w, h) = {
            let mut label_guard = label.lock().expect("label lock");
            (
                label_guard.shape().position_ref().x,
                label_guard.shape().position_ref().y,
                label_guard.shape().size_ref().x,
                label_guard.shape().size_ref().y,
            )
        };
        assert!(y + h < node_y, "north self-loop labels should be above node");
        label_centers.push(x + w / 2.0);
    }

    label_centers.sort_by(|left, right| left.total_cmp(right));
    assert!(
        label_centers[0] < node_center_x - 5.0 && label_centers[1] > node_center_x + 5.0,
        "sequenced ordering should place one label left and one label right of node center"
    );
}

#[test]
fn self_loop_router_expands_node_margin_for_self_loop_labels() {
    init_layered_metadata();
    let (graph, node, edges) = build_parallel_north_self_loop_graph_with_labels();

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    let layer = Layer::new(&graph);
    {
        graph.lock()
            .expect("graph lock")
            .layers_mut()
            .push(layer.clone());
    }
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);
    let mut router = SelfLoopRouter;
    run_processor(&mut router, &graph);

    let margin_top = {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.margin().top
    };

    let first_label = {
        let edge_guard = edges[0].lock().expect("edge lock");
        edge_guard
            .labels()
            .first()
            .expect("expected first self-loop label")
            .clone()
    };
    let label_top_local = {
        let mut label_guard = first_label.lock().expect("label lock");
        label_guard.shape().position_ref().y
    };

    assert!(
        margin_top + 1e-6 >= -label_top_local,
        "node top margin should cover self-loop label: margin_top={margin_top}, label_top_local={label_top_local}"
    );
}

#[test]
fn self_loop_router_offsets_corner_for_inline_label_clearance() {
    init_layered_metadata();
    let (regular_graph, regular_node, regular_edge) =
        build_north_south_self_loop_graph_with_label(false);
    let (inline_graph, inline_node, inline_edge) =
        build_north_south_self_loop_graph_with_label(true);

    let inline_flag_before = {
        let edge_guard = inline_edge.lock().expect("inline edge lock");
        let label = edge_guard
            .labels()
            .first()
            .expect("inline edge label")
            .clone();
        label
            .lock()
            .ok()
            .and_then(|mut label_guard| label_guard.get_property(LayeredOptions::EDGE_LABELS_INLINE))
            .unwrap_or(false)
    };
    assert!(inline_flag_before, "inline label flag must be set before routing");

    for (graph, node) in [(&regular_graph, &regular_node), (&inline_graph, &inline_node)] {
        let mut pre = SelfLoopPreProcessor;
        run_processor(&mut pre, graph);

        let layer = Layer::new(graph);
        graph
            .lock()
            .expect("graph lock")
            .layers_mut()
            .push(layer.clone());
        LNode::set_layer(node, Some(layer));
        graph
            .lock()
            .expect("graph lock")
            .layerless_nodes_mut()
            .retain(|candidate| !Arc::ptr_eq(candidate, node));

        let mut restorer = SelfLoopPortRestorer;
        run_processor(&mut restorer, graph);
        let mut router = SelfLoopRouter;
        run_processor(&mut router, graph);
        let mut post = SelfLoopPostProcessor;
        run_processor(&mut post, graph);
    }

    let (inline_self_loop_type, label_side) = inline_node
        .lock()
        .expect("inline node lock")
        .get_property(InternalProperties::SELF_LOOP_HOLDER)
        .and_then(|holder| {
            holder
                .lock()
                .ok()
                .and_then(|holder_guard| holder_guard.sl_hyper_loops().first().cloned())
        })
        .and_then(|sl_loop| {
            sl_loop
                .lock()
                .ok()
                .and_then(|sl_loop_guard| {
                    sl_loop_guard
                        .sl_labels()
                        .map(|labels| (sl_loop_guard.self_loop_type(), labels.side()))
                })
        })
        .unwrap_or((None, PortSide::Undefined));

    assert_eq!(
        inline_self_loop_type,
        Some(SelfLoopType::TwoSidesOpposing),
        "expected north/south loop to be classified as TwoSidesOpposing"
    );

    let regular_extreme = {
        let edge_guard = regular_edge.lock().expect("regular edge lock");
        match label_side {
            PortSide::North => edge_guard
                .bend_points_ref()
                .iter()
                .map(|point| point.y)
                .fold(f64::INFINITY, f64::min),
            PortSide::South => edge_guard
                .bend_points_ref()
                .iter()
                .map(|point| point.y)
                .fold(f64::NEG_INFINITY, f64::max),
            PortSide::East => edge_guard
                .bend_points_ref()
                .iter()
                .map(|point| point.x)
                .fold(f64::NEG_INFINITY, f64::max),
            PortSide::West => edge_guard
                .bend_points_ref()
                .iter()
                .map(|point| point.x)
                .fold(f64::INFINITY, f64::min),
            PortSide::Undefined => f64::NAN,
        }
    };
    let inline_extreme = {
        let edge_guard = inline_edge.lock().expect("inline edge lock");
        match label_side {
            PortSide::North => edge_guard
                .bend_points_ref()
                .iter()
                .map(|point| point.y)
                .fold(f64::INFINITY, f64::min),
            PortSide::South => edge_guard
                .bend_points_ref()
                .iter()
                .map(|point| point.y)
                .fold(f64::NEG_INFINITY, f64::max),
            PortSide::East => edge_guard
                .bend_points_ref()
                .iter()
                .map(|point| point.x)
                .fold(f64::NEG_INFINITY, f64::max),
            PortSide::West => edge_guard
                .bend_points_ref()
                .iter()
                .map(|point| point.x)
                .fold(f64::INFINITY, f64::min),
            PortSide::Undefined => f64::NAN,
        }
    };

    assert!(regular_extreme.is_finite() && inline_extreme.is_finite());
    match label_side {
        PortSide::North | PortSide::West => assert!(
            inline_extreme < regular_extreme - 3.0,
            "inline label should push corner farther outward for {label_side:?}: inline={inline_extreme}, regular={regular_extreme}"
        ),
        PortSide::South | PortSide::East => assert!(
            inline_extreme > regular_extreme + 3.0,
            "inline label should push corner farther outward for {label_side:?}: inline={inline_extreme}, regular={regular_extreme}"
        ),
        PortSide::Undefined => panic!("label side should be assigned for inline self-loop"),
    }
}

#[test]
fn self_loop_router_prefers_west_route_when_east_ports_are_more_connected() {
    init_layered_metadata();
    let (graph, node, self_loop) = build_opposing_self_loop_with_side_penalty(true, false);

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);
    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));
    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);
    let mut router = SelfLoopRouter;
    run_processor(&mut router, &graph);
    let mut post = SelfLoopPostProcessor;
    run_processor(&mut post, &graph);

    let (node_x, _) = {
        let mut node_guard = node.lock().expect("node lock");
        (
            node_guard.shape().position_ref().x,
            node_guard.shape().size_ref().x,
        )
    };
    let min_x = {
        let edge_guard = self_loop.lock().expect("self-loop edge lock");
        edge_guard
            .bend_points_ref()
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min)
    };
    assert!(
        min_x < node_x - 1e-6,
        "expected west route when east side is more connected, min_x={min_x}, node_x={node_x}"
    );
}

#[test]
fn self_loop_router_prefers_east_route_when_west_ports_are_more_connected() {
    init_layered_metadata();
    let (graph, node, self_loop) = build_opposing_self_loop_with_side_penalty(false, true);

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);
    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));
    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);
    let mut router = SelfLoopRouter;
    run_processor(&mut router, &graph);
    let mut post = SelfLoopPostProcessor;
    run_processor(&mut post, &graph);

    let (node_x, node_w) = {
        let mut node_guard = node.lock().expect("node lock");
        (
            node_guard.shape().position_ref().x,
            node_guard.shape().size_ref().x,
        )
    };
    let max_x = {
        let edge_guard = self_loop.lock().expect("self-loop edge lock");
        edge_guard
            .bend_points_ref()
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max)
    };
    assert!(
        max_x > node_x + node_w + 1e-6,
        "expected east route when west side is more connected, max_x={max_x}, node_right={}",
        node_x + node_w
    );
}

#[test]
fn self_loop_port_restorer_keeps_north_west_corner_clockwise_order() {
    init_layered_metadata();
    let (graph, node, hidden_north, hidden_west, visible_north, visible_west) =
        build_north_west_corner_mixed_graph();

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);

    let ports = node.lock().expect("node lock").ports().clone();
    let index_of = |target: &LPortRef| {
        ports
            .iter()
            .position(|port| Arc::ptr_eq(port, target))
            .expect("port should be in restored list")
    };

    let hidden_north_index = index_of(&hidden_north);
    let visible_north_index = index_of(&visible_north);
    let hidden_west_index = index_of(&hidden_west);
    let visible_west_index = index_of(&visible_west);

    assert!(
        hidden_north_index < visible_north_index,
        "north corner port must be restored in NORTH/START before regular north ports"
    );
    assert!(
        hidden_west_index > visible_west_index,
        "west corner port must be restored in WEST/END after regular west ports"
    );
}

#[test]
fn self_loop_port_restorer_equally_distribution_assigns_corner_by_net_flow() {
    init_layered_metadata();
    let (graph, node, small_loop_source, small_loop_target) =
        build_equally_distribution_graph_with_unique_loop_sizes();

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);

    let holder = node
        .lock()
        .expect("node lock")
        .get_property(InternalProperties::SELF_LOOP_HOLDER)
        .expect("self loop holder");

    let loop_sizes = holder
        .lock()
        .expect("holder lock")
        .sl_hyper_loops()
        .iter()
        .map(|sl_loop| sl_loop.lock().expect("loop lock").sl_ports().len())
        .collect::<Vec<_>>();
    assert!(
        loop_sizes.contains(&2)
            && loop_sizes.contains(&3)
            && loop_sizes.contains(&4)
            && loop_sizes.contains(&5)
            && loop_sizes.contains(&6),
        "expected five self hyper loops with unique sizes, got {loop_sizes:?}"
    );

    let source_side = small_loop_source.lock().expect("source lock").side();
    let target_side = small_loop_target.lock().expect("target lock").side();
    let small_loop_sides = [source_side, target_side];

    assert!(
        small_loop_sides.contains(&PortSide::West) && small_loop_sides.contains(&PortSide::North),
        "size-2 loop assigned to NORTH_WEST_CORNER target should be split across WEST/NORTH, got {small_loop_sides:?}"
    );

    let side_set = node
        .lock()
        .expect("node lock")
        .ports()
        .iter()
        .map(|port| port.lock().expect("port lock").side())
        .collect::<std::collections::HashSet<_>>();
    assert!(
        side_set.contains(&PortSide::North)
            && side_set.contains(&PortSide::East)
            && side_set.contains(&PortSide::South)
            && side_set.contains(&PortSide::West),
        "EQUALLY distribution should cover all four sides, got {side_set:?}"
    );
}

#[test]
fn self_loop_port_restorer_places_east_connected_south_before_middle_hidden_south() {
    init_layered_metadata();
    let (graph, node, visible_south, hidden_south_ports) = build_south_middle_with_east_dummy_graph();

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);

    let ports = node.lock().expect("node lock").ports().clone();
    let visible_index = ports
        .iter()
        .position(|port| Arc::ptr_eq(port, &visible_south))
        .expect("visible south port must stay on node");
    let hidden_indices = hidden_south_ports
        .iter()
        .map(|hidden| {
            ports
                .iter()
                .position(|port| Arc::ptr_eq(port, hidden))
                .expect("hidden south port should be restored")
        })
        .collect::<Vec<_>>();

    assert!(
        hidden_indices.iter().all(|index| visible_index < *index),
        "south port with EAST dummy-connection should be ordered before SOUTH/MIDDLE restored ports: visible={visible_index}, hidden={hidden_indices:?}"
    );
}

#[test]
fn self_loop_port_restorer_places_west_connected_north_before_middle_hidden_north() {
    init_layered_metadata();
    let (graph, node, visible_north, hidden_north_ports) = build_north_middle_with_west_dummy_graph();

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);

    let ports = node.lock().expect("node lock").ports().clone();
    let visible_index = ports
        .iter()
        .position(|port| Arc::ptr_eq(port, &visible_north))
        .expect("visible north port must stay on node");
    let hidden_indices = hidden_north_ports
        .iter()
        .map(|hidden| {
            ports
                .iter()
                .position(|port| Arc::ptr_eq(port, hidden))
                .expect("hidden north port should be restored")
        })
        .collect::<Vec<_>>();

    assert!(
        hidden_indices.iter().all(|index| visible_index < *index),
        "north port with WEST dummy-connection should be ordered before NORTH/MIDDLE restored ports: visible={visible_index}, hidden={hidden_indices:?}"
    );
}

#[test]
fn self_loop_port_restorer_orders_corner_and_dummy_connected_ports_before_middle_groups() {
    init_layered_metadata();
    let (
        graph,
        node,
        visible_north,
        visible_south,
        hidden_north_corner,
        hidden_north_middle_ports,
        hidden_south_middle_ports,
        _,
    ) = build_corner_and_dummy_combined_graph();

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);

    let ports = node.lock().expect("node lock").ports().clone();
    let index_of = |target: &LPortRef| {
        ports
            .iter()
            .position(|port| Arc::ptr_eq(port, target))
            .expect("target port should be present after restore")
    };

    let north_corner_index = index_of(&hidden_north_corner);
    let visible_north_index = index_of(&visible_north);
    let north_middle_indices = hidden_north_middle_ports
        .iter()
        .map(index_of)
        .collect::<Vec<_>>();

    assert!(
        north_corner_index < visible_north_index,
        "north-west corner port should be restored before north dummy-connected visible port: corner={north_corner_index}, visible={visible_north_index}"
    );
    assert!(
        north_middle_indices
            .iter()
            .all(|index| visible_north_index < *index),
        "north dummy-connected visible port should be before NORTH/MIDDLE hidden ports: visible={visible_north_index}, hidden={north_middle_indices:?}"
    );

    let visible_south_index = index_of(&visible_south);
    let south_middle_indices = hidden_south_middle_ports
        .iter()
        .map(index_of)
        .collect::<Vec<_>>();
    assert!(
        south_middle_indices
            .iter()
            .all(|index| visible_south_index < *index),
        "south dummy-connected visible port should be before SOUTH/MIDDLE hidden ports: visible={visible_south_index}, hidden={south_middle_indices:?}"
    );
}

#[test]
fn self_loop_router_routes_corner_and_dummy_combined_loops_with_labels() {
    init_layered_metadata();
    let (
        graph,
        node,
        _visible_north,
        _visible_south,
        _hidden_north_corner,
        _hidden_north_middle_ports,
        _hidden_south_middle_ports,
        self_loop_edges,
    ) = build_corner_and_dummy_combined_graph();

    let mut pre = SelfLoopPreProcessor;
    run_processor(&mut pre, &graph);

    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    LNode::set_layer(&node, Some(layer));
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .retain(|candidate| !Arc::ptr_eq(candidate, &node));

    let mut restorer = SelfLoopPortRestorer;
    run_processor(&mut restorer, &graph);
    let mut router = SelfLoopRouter;
    run_processor(&mut router, &graph);
    let mut post = SelfLoopPostProcessor;
    run_processor(&mut post, &graph);

    let (node_x, node_y, node_w, node_h) = {
        let mut node_guard = node.lock().expect("node lock");
        (
            node_guard.shape().position_ref().x,
            node_guard.shape().position_ref().y,
            node_guard.shape().size_ref().x,
            node_guard.shape().size_ref().y,
        )
    };

    for edge in self_loop_edges {
        let edge_guard = edge.lock().expect("edge lock");
        assert_eq!(
            edge_guard.labels().len(),
            1,
            "self-loop label should stay attached across pre/restore/route/post"
        );
        assert!(
            !edge_guard.bend_points_ref().is_empty(),
            "self-loop should receive routed bend points"
        );

        let has_outside_point = edge_guard.bend_points_ref().iter().any(|point| {
            point.x < node_x
                || point.x > node_x + node_w
                || point.y < node_y
                || point.y > node_y + node_h
        });
        assert!(
            has_outside_point,
            "self-loop should route outside node bounds for corner/dummy mix"
        );
    }
}
