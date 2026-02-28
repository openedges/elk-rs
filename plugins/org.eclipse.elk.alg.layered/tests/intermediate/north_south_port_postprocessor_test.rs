use std::sync::Arc;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::{
    NorthSouthPortPostprocessor, NorthSouthPortPreprocessor,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn init_layered_metadata() {
    LayoutMetaDataService::get_instance()
        .register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn graph_with_single_layer() -> (LGraphRef, Arc<Mutex<Layer>>) {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    (graph, layer)
}

fn add_node(graph: &LGraphRef, layer: &Arc<Mutex<Layer>>) -> LNodeRef {
    let node = LNode::new(graph);
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port(node: &LNodeRef, side: PortSide) -> LPortRef {
    let port = LPort::new();
    {
        let mut port_guard = port.lock().expect("port lock");
        port_guard.set_side(side);
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn connect(source: &LPortRef, target: &LPortRef) {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
}

#[test]
fn north_south_postprocessor_removes_all_north_south_dummy_nodes() {
    init_layered_metadata();
    let (graph, layer) = graph_with_single_layer();

    let owner = add_node(&graph, &layer);
    owner.lock().expect("owner lock").set_property(
        LayeredOptions::PORT_CONSTRAINTS,
        Some(PortConstraints::FixedSide),
    );

    let north = add_port(&owner, PortSide::North);
    let south = add_port(&owner, PortSide::South);

    let other = add_node(&graph, &layer);
    let other_east = add_port(&other, PortSide::East);
    let other_west = add_port(&other, PortSide::West);
    connect(&other_east, &north);
    connect(&south, &other_west);

    let mut monitor = NullElkProgressMonitor;
    NorthSouthPortPreprocessor.process(&mut graph.lock().expect("graph lock"), &mut monitor);
    NorthSouthPortPostprocessor.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let has_ns_dummy = graph
        .lock()
        .expect("graph lock")
        .layers()
        .iter()
        .flat_map(|layer| {
            layer
                .lock()
                .expect("layer lock")
                .nodes()
                .clone()
                .into_iter()
        })
        .any(|node| {
            node.lock()
                .ok()
                .map(|node_guard| node_guard.node_type() == NodeType::NorthSouthPort)
                .unwrap_or(false)
        });

    assert!(
        !has_ns_dummy,
        "all north/south port dummy nodes must be removed"
    );
}
