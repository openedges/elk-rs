mod issue_support;

use issue_support::{
    create_edge, create_graph, create_node, init_layered_options, run_layout, set_node_property,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayerUnzippingStrategy, LayeredOptions, OrderingStrategy,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef;

fn run_alternating_case(middle_count: usize, split: i32, with_source: bool, with_sink: bool) {
    init_layered_options();
    let graph = create_graph();
    set_node_property(
        &graph,
        LayeredOptions::LAYER_UNZIPPING_STRATEGY,
        LayerUnzippingStrategy::Alternating,
    );
    set_node_property(
        &graph,
        LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY,
        OrderingStrategy::PreferEdges,
    );

    let source = if with_source {
        Some(create_node(&graph, 30.0, 30.0))
    } else {
        None
    };
    let sink = if with_sink {
        Some(create_node(&graph, 30.0, 30.0))
    } else {
        None
    };

    let mut middle = Vec::with_capacity(middle_count);
    for _ in 0..middle_count {
        middle.push(create_node(&graph, 30.0, 30.0));
    }
    if let Some(first) = middle.first() {
        set_node_property(first, LayeredOptions::LAYER_UNZIPPING_LAYER_SPLIT, split);
    }

    if let Some(source) = source.as_ref() {
        for node in &middle {
            create_edge(
                ElkConnectableShapeRef::Node(source.clone()),
                ElkConnectableShapeRef::Node(node.clone()),
            );
        }
    }
    if let Some(sink) = sink.as_ref() {
        for node in &middle {
            create_edge(
                ElkConnectableShapeRef::Node(node.clone()),
                ElkConnectableShapeRef::Node(sink.clone()),
            );
        }
    }

    run_layout(&graph);

    if let Some(source) = source {
        let source_x = source.borrow_mut().connectable().shape().x();
        for node in &middle {
            let x = node.borrow_mut().connectable().shape().x();
            assert!(x.is_finite());
            assert!(source_x <= x || (source_x - x).abs() < 1e-6);
        }
    }
    if let Some(sink) = sink {
        let sink_x = sink.borrow_mut().connectable().shape().x();
        for node in &middle {
            let x = node.borrow_mut().connectable().shape().x();
            assert!(x.is_finite());
            assert!(x <= sink_x || (x - sink_x).abs() < 1e-6);
        }
    }
}

#[test]
fn simple_two_split() {
    run_alternating_case(3, 2, true, true);
}

#[test]
fn simple_three_split() {
    run_alternating_case(4, 3, true, true);
}

#[test]
fn dangling_outgoing() {
    run_alternating_case(4, 2, true, false);
}

#[test]
fn dangling_incoming() {
    run_alternating_case(4, 2, false, true);
}

#[test]
fn multiple_layers_split() {
    run_alternating_case(6, 2, true, true);
}

#[test]
fn multiple_incoming_edges() {
    run_alternating_case(3, 2, true, true);
}

#[test]
fn multiple_outgoing_edges() {
    run_alternating_case(3, 2, true, true);
}

#[test]
fn mixed_dangling_incoming() {
    run_alternating_case(5, 2, true, true);
}

#[test]
fn mixed_dangling_outgoing() {
    run_alternating_case(5, 2, true, true);
}

#[test]
fn split_factor_four() {
    run_alternating_case(5, 4, true, true);
}

#[test]
fn split_factor_five() {
    run_alternating_case(6, 5, true, true);
}

#[test]
fn split_factor_with_single_middle_node() {
    run_alternating_case(1, 2, true, true);
}
