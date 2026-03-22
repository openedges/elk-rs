use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::{
    IntermediateProcessorStrategy, NorthSouthPortPostprocessor, NorthSouthPortPreprocessor,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor_factory::ILayoutProcessorFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn new_graph_with_single_layer() -> (LGraphRef, LayerRef) {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    {
        let mut graph_guard = graph.lock();        graph_guard.layers_mut().push(layer.clone());
    }
    (graph, layer)
}

fn init_layered_metadata() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn add_node(graph: &LGraphRef, layer: &LayerRef, x: f64, y: f64) -> LNodeRef {
    let node = LNode::new(graph);
    {
        let mut node_guard = node.lock();        node_guard.shape().position().x = x;
        node_guard.shape().position().y = y;
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port(node: &LNodeRef, side: PortSide, x: f64, y: f64) -> LPortRef {
    let port = LPort::new();
    {
        let mut port_guard = port.lock();        port_guard.set_side(side);
        port_guard.shape().position().x = x;
        port_guard.shape().position().y = y;
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn connect(source: &LPortRef, target: &LPortRef) -> LEdgeRef {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
    edge
}

struct NorthSouthFixture {
    graph: LGraphRef,
    layer: LayerRef,
    owner: LNodeRef,
    north_port: LPortRef,
    south_port: LPortRef,
    incoming_edge: LEdgeRef,
    outgoing_edge: LEdgeRef,
}

fn build_north_south_fixture() -> NorthSouthFixture {
    let (graph, layer) = new_graph_with_single_layer();
    let owner = add_node(&graph, &layer, 20.0, 40.0);
    {
        let mut owner_guard = owner.lock();        owner_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedSide),
        );
    }

    let north_port = add_port(&owner, PortSide::North, 5.0, -2.0);
    let south_port = add_port(&owner, PortSide::South, 7.0, 12.0);

    let other = add_node(&graph, &layer, 100.0, 40.0);
    let other_east = add_port(&other, PortSide::East, 0.0, 3.0);
    let other_west = add_port(&other, PortSide::West, 0.0, 9.0);

    let incoming_edge = connect(&other_east, &north_port);
    let outgoing_edge = connect(&south_port, &other_west);

    NorthSouthFixture {
        graph,
        layer,
        owner,
        north_port,
        south_port,
        incoming_edge,
        outgoing_edge,
    }
}

fn run_preprocessor(graph: &LGraphRef) {
    init_layered_metadata();
    let mut processor = NorthSouthPortPreprocessor;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock();    processor.process(&mut graph_guard, &mut monitor);
}

fn run_postprocessor(graph: &LGraphRef) {
    init_layered_metadata();
    let mut processor = NorthSouthPortPostprocessor;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock();    processor.process(&mut graph_guard, &mut monitor);
}

fn layer_node_count(layer: &LayerRef) -> usize {
    layer.lock().nodes().len()
}

fn dummy_nodes_in_layer(layer: &LayerRef) -> Vec<LNodeRef> {
    layer
        .lock()
        
        .nodes()
        .iter()
        .filter_map(|node| {
            let node_type = node.lock().node_type();
            if node_type == NodeType::NorthSouthPort {
                Some(node.clone())
            } else {
                None
            }
        })
        .collect()
}

fn snapshot(graph: &LGraphRef) -> (usize, usize, usize, usize) {
    let graph_guard = graph.lock();    let layer_count = graph_guard.layers().len();
    let first_layer_nodes = graph_guard
        .layers()
        .first()
        .map(|layer| layer.lock().nodes().len())
        .unwrap_or(0);
    let layerless_count = graph_guard.layerless_nodes().len();
    let normal_nodes = graph_guard
        .layers()
        .iter()
        .map(|layer| layer.lock())
        .map(|layer_guard| {
            layer_guard
                .nodes()
                .iter()
                .filter(|node| {
                    node.lock().node_type() == NodeType::Normal
                })
                .count()
        })
        .sum();
    (
        layer_count,
        first_layer_nodes,
        layerless_count,
        normal_nodes,
    )
}

fn run_strategy(strategy: IntermediateProcessorStrategy, graph: &LGraphRef) {
    init_layered_metadata();
    let mut processor = strategy.create();
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock();    processor.process(&mut graph_guard, &mut monitor);
}

fn assert_strategy_stable(strategy: IntermediateProcessorStrategy) {
    let (graph, layer) = new_graph_with_single_layer();
    let _node = add_node(&graph, &layer, 0.0, 0.0);
    let before = snapshot(&graph);
    run_strategy(strategy, &graph);
    assert_eq!(snapshot(&graph), before);
}

#[test]
fn north_south_preprocessor_creates_dummy_nodes_for_connected_ports() {
    let fixture = build_north_south_fixture();
    let before_count = layer_node_count(&fixture.layer);
    run_preprocessor(&fixture.graph);
    let after_count = layer_node_count(&fixture.layer);

    assert_eq!(after_count, before_count + 2);
    assert_eq!(dummy_nodes_in_layer(&fixture.layer).len(), 2);
}

#[test]
fn north_south_preprocessor_skips_unconnected_ports() {
    let (graph, layer) = new_graph_with_single_layer();
    let owner = add_node(&graph, &layer, 0.0, 0.0);
    {
        let mut owner_guard = owner.lock();        owner_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedSide),
        );
    }
    let _north = add_port(&owner, PortSide::North, 0.0, 0.0);
    let _south = add_port(&owner, PortSide::South, 0.0, 0.0);

    run_preprocessor(&graph);
    assert_eq!(dummy_nodes_in_layer(&layer).len(), 0);
}

#[test]
fn north_south_preprocessor_sets_layout_unit_and_barycenter_associates() {
    let fixture = build_north_south_fixture();
    run_preprocessor(&fixture.graph);

    let barycenter = fixture
        .owner
        .lock()
        
        .get_property(InternalProperties::BARYCENTER_ASSOCIATES)
        .unwrap_or_default();
    assert_eq!(barycenter.len(), 2);

    let dummies = dummy_nodes_in_layer(&fixture.layer);
    for dummy in dummies {
        let layout_unit = dummy
            .lock()
            
            .get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT)
            .expect("layout unit");
        assert!(Arc::ptr_eq(&layout_unit, &fixture.owner));
    }
}

#[test]
fn north_south_postprocessor_removes_dummy_nodes_from_layers() {
    let fixture = build_north_south_fixture();
    run_preprocessor(&fixture.graph);
    assert!(!dummy_nodes_in_layer(&fixture.layer).is_empty());

    run_postprocessor(&fixture.graph);
    assert!(dummy_nodes_in_layer(&fixture.layer).is_empty());
}

#[test]
fn north_south_postprocessor_restores_incoming_edge_to_origin_port() {
    let fixture = build_north_south_fixture();
    run_preprocessor(&fixture.graph);
    run_postprocessor(&fixture.graph);

    let target = fixture
        .incoming_edge
        .lock()
        
        .target()
        .expect("target");
    assert!(Arc::ptr_eq(&target, &fixture.north_port));
    let bend_count = fixture
        .incoming_edge
        .lock()
        
        .bend_points_ref()
        .size();
    assert!(bend_count >= 1);
}

#[test]
fn north_south_postprocessor_restores_outgoing_edge_to_origin_port() {
    let fixture = build_north_south_fixture();
    run_preprocessor(&fixture.graph);
    run_postprocessor(&fixture.graph);

    let source = fixture
        .outgoing_edge
        .lock()
        
        .source()
        .expect("source");
    assert!(Arc::ptr_eq(&source, &fixture.south_port));
    let bend_count = fixture
        .outgoing_edge
        .lock()
        
        .bend_points_ref()
        .size();
    assert!(bend_count >= 1);
}

#[test]
fn sort_by_input_order_of_model_strategy_is_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::SortByInputOrderOfModel);
}

#[test]
fn port_list_sorter_strategy_is_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::PortListSorter);
}

#[test]
fn layer_constraint_processor_strategies_are_stable_on_simple_graph() {
    let (graph, layer) = new_graph_with_single_layer();
    let _node = add_node(&graph, &layer, 0.0, 0.0);
    let before = snapshot(&graph);

    run_strategy(
        IntermediateProcessorStrategy::LayerConstraintPreprocessor,
        &graph,
    );
    run_strategy(
        IntermediateProcessorStrategy::LayerConstraintPostprocessor,
        &graph,
    );
    assert_eq!(snapshot(&graph), before);
}

#[test]
fn partition_postprocessor_strategy_is_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::PartitionPostprocessor);
}

#[test]
fn long_edge_strategies_are_stable_on_simple_graph() {
    let (graph, layer) = new_graph_with_single_layer();
    let _node = add_node(&graph, &layer, 0.0, 0.0);
    let before = snapshot(&graph);

    run_strategy(IntermediateProcessorStrategy::LongEdgeSplitter, &graph);
    run_strategy(IntermediateProcessorStrategy::LongEdgeJoiner, &graph);
    assert_eq!(snapshot(&graph), before);
}

#[test]
fn end_label_and_reversed_edge_strategies_are_stable_on_simple_graph() {
    let (graph, layer) = new_graph_with_single_layer();
    let _node = add_node(&graph, &layer, 0.0, 0.0);
    let before = snapshot(&graph);

    run_strategy(IntermediateProcessorStrategy::EndLabelSorter, &graph);
    run_strategy(IntermediateProcessorStrategy::ReversedEdgeRestorer, &graph);
    assert_eq!(snapshot(&graph), before);
}

#[test]
fn edge_and_layer_constraint_edge_reverser_strategy_is_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::EdgeAndLayerConstraintEdgeReverser);
}

#[test]
fn partition_pre_and_mid_strategies_are_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::PartitionPreprocessor);
    assert_strategy_stable(IntermediateProcessorStrategy::PartitionMidprocessor);
}

#[test]
fn in_layer_constraint_processor_strategy_is_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::InLayerConstraintProcessor);
}

#[test]
fn inverted_port_processor_strategy_is_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::InvertedPortProcessor);
}

#[test]
fn port_side_processor_strategy_is_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::PortSideProcessor);
}

#[test]
fn label_side_selector_strategy_is_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::LabelSideSelector);
}

#[test]
fn comment_postprocessor_strategy_is_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::CommentPostprocessor);
}

#[test]
fn comment_preprocessor_strategy_is_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::CommentPreprocessor);
}

#[test]
fn comment_node_margin_calculator_strategy_is_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::CommentNodeMarginCalculator);
}

#[test]
fn end_label_pre_and_postprocessor_strategies_are_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::EndLabelPreprocessor);
    assert_strategy_stable(IntermediateProcessorStrategy::EndLabelPostprocessor);
}

#[test]
fn long_edge_splitter_and_joiner_individual_strategies_are_stable_on_simple_graph() {
    assert_strategy_stable(IntermediateProcessorStrategy::LongEdgeSplitter);
    assert_strategy_stable(IntermediateProcessorStrategy::LongEdgeJoiner);
}
