use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::SortByInputModelProcessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredMetaDataProvider, LayeredOptions, LongEdgeOrderingStrategy,
    OrderingStrategy,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn graph_with_layers(count: usize) -> (LGraphRef, Vec<LayerRef>) {
    init_layered_metadata();
    let graph = LGraph::new();
    let mut layers = Vec::with_capacity(count);
    {
        let mut graph_guard = graph.lock();        for _ in 0..count {
            let layer = Layer::new(&graph);
            graph_guard.layers_mut().push(layer.clone());
            layers.push(layer);
        }
        graph_guard.set_property(
            LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY,
            Some(OrderingStrategy::NodesAndEdges),
        );
        graph_guard.set_property(
            LayeredOptions::CONSIDER_MODEL_ORDER_LONG_EDGE_STRATEGY,
            Some(LongEdgeOrderingStrategy::Equal),
        );
        graph_guard.set_property(
            LayeredOptions::CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER,
            Some(false),
        );
    }
    (graph, layers)
}

fn init_layered_metadata() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn add_node(graph: &LGraphRef, layer: &LayerRef, model_order: i32) -> LNodeRef {
    let node = LNode::new(graph);
    {
        let mut node_guard = node.lock();        node_guard.set_property(InternalProperties::MODEL_ORDER, Some(model_order));
    }
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

fn connect_with_model_order(source: &LPortRef, target: &LPortRef, model_order: i32) {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
    {
        let mut edge_guard = edge.lock();
        edge_guard.set_property(InternalProperties::MODEL_ORDER, Some(model_order));
    };
}

fn layer_nodes(layer: &LayerRef) -> Vec<LNodeRef> {
    layer.lock().nodes().clone()
}

fn node_ports(node: &LNodeRef) -> Vec<LPortRef> {
    node.lock().ports().clone()
}

fn run_sorter_direct(graph: &LGraphRef) {
    let mut processor = SortByInputModelProcessor;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock();    processor.process(&mut graph_guard, &mut monitor);
}

#[test]
fn sort_by_input_model_processor_sorts_nodes_by_model_order() {
    let (graph, layers) = graph_with_layers(1);
    let layer = layers[0].clone();

    let n2 = add_node(&graph, &layer, 2);
    let n0 = add_node(&graph, &layer, 0);
    let n1 = add_node(&graph, &layer, 1);

    run_sorter_direct(&graph);

    let nodes = layer_nodes(&layer);
    assert!(Arc::ptr_eq(&nodes[0], &n0));
    assert!(Arc::ptr_eq(&nodes[1], &n1));
    assert!(Arc::ptr_eq(&nodes[2], &n2));
}

#[test]
fn sort_by_input_model_processor_sorts_ports_by_edge_model_order() {
    let (graph, layers) = graph_with_layers(2);
    let source_layer = layers[0].clone();
    let target_layer = layers[1].clone();

    let source = add_node(&graph, &source_layer, 0);
    {
        let mut source_guard = source.lock();        source_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::Free),
        );
    }
    let p_high = add_port(&source, PortSide::East);
    let p_low = add_port(&source, PortSide::East);

    let t_low = add_node(&graph, &target_layer, 0);
    let t_high = add_node(&graph, &target_layer, 1);
    let t_low_west = add_port(&t_low, PortSide::West);
    let t_high_west = add_port(&t_high, PortSide::West);

    connect_with_model_order(&p_high, &t_high_west, 10);
    connect_with_model_order(&p_low, &t_low_west, 1);

    run_sorter_direct(&graph);

    let ports = node_ports(&source);
    assert!(Arc::ptr_eq(&ports[0], &p_low));
    assert!(Arc::ptr_eq(&ports[1], &p_high));
}
