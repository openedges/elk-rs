use std::sync::Arc;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::NorthSouthPortPreprocessor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
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
        let mut port_guard = port.lock();        port_guard.set_side(side);
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
fn north_south_preprocessor_isolates_north_south_ports() {
    init_layered_metadata();
    let (graph, layer) = graph_with_single_layer();

    let owner = add_node(&graph, &layer);
    owner.lock().set_property(
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

    let mut processor = NorthSouthPortPreprocessor;
    let mut monitor = NullElkProgressMonitor;
    processor.process(&mut graph.lock(), &mut monitor);

    let owner_ports = owner.lock().ports().clone();
    for port in owner_ports {
        let (side, connected) = {
            let port_guard = port.lock();
            (port_guard.side(), !port_guard.connected_edges().is_empty())
        };
        if side == PortSide::North || side == PortSide::South {
            assert!(
                !connected,
                "north/south port should be isolated after preprocessing"
            );
        }
    }
}

#[test]
fn north_south_preprocessor_sets_layout_unit_for_fixed_side_node_without_north_south_ports() {
    init_layered_metadata();
    let (graph, layer) = graph_with_single_layer();

    let owner = add_node(&graph, &layer);
    owner.lock().set_property(
        LayeredOptions::PORT_CONSTRAINTS,
        Some(PortConstraints::FixedSide),
    );
    add_port(&owner, PortSide::East);
    add_port(&owner, PortSide::West);

    let mut processor = NorthSouthPortPreprocessor;
    let mut monitor = NullElkProgressMonitor;
    processor.process(&mut graph.lock(), &mut monitor);

    let layout_unit = owner
        .lock()
        
        .get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT)
        .expect("layout unit should be set");
    assert!(
        std::sync::Arc::ptr_eq(&layout_unit, &owner),
        "layout unit should point to the owner node"
    );
}
