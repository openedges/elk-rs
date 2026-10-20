
use crate::common::issue_support::{
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

#[test]
fn exactly_enough_nodes_fill_all_sublayers() {
    init_layered_options();
    use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{LGraph, LNode, Layer};
    use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::AlternatingLayerUnzipper;
    use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
    use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

    for split in [2, 3, 4] {
        for count in [split - 1, split] {
            let graph = LGraph::new();
            let layer = Layer::new(&graph);
            graph.lock().layers_mut().push(layer.clone());
            let mut nodes = Vec::new();
            for _ in 0..count {
                let node = LNode::new(&graph);
                node.lock().set_property(LayeredOptions::LAYER_UNZIPPING_LAYER_SPLIT, Some(split));
                LNode::set_layer(&node, Some(layer.clone()));
                nodes.push(node);
            }
            {
                let mut graph = graph.lock();
                AlternatingLayerUnzipper.process(&mut graph, &mut NullElkProgressMonitor);
                assert_eq!(graph.layers().len(), if count == split { split as usize } else { 1 });
            }
            if count == split {
                for pair in nodes.windows(2) {
                    let left = pair[0].lock().layer().unwrap();
                    let right = pair[1].lock().layer().unwrap();
                    assert!(!std::sync::Arc::ptr_eq(&left, &right));
                }
            }
        }
    }
}
