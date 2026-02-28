use std::collections::HashSet;
use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LNode, LNodeRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::InternalProperties;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::forster_constraint_resolver::ForsterConstraintResolver;

#[test]
fn test_successor_constraints() {
    let (graph, layer) = create_graph_with_layer();
    let a = create_node(&graph, &layer);
    let b = create_node(&graph, &layer);
    let c = create_node(&graph, &layer);

    if let Ok(mut a_guard) = a.lock() {
        a_guard.set_property(
            InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS,
            Some(vec![b.clone()]),
        );
    }

    let mut node_order = vec![b.clone(), a.clone(), c.clone()];
    let mut resolver = prepare_resolver(&node_order);
    resolver.process_constraints(&mut node_order);

    assert_successor_constraints_respected(&node_order);
}

#[test]
fn test_non_overlapping_layout_units() {
    let (graph, layer) = create_graph_with_layer();
    let a1 = create_node(&graph, &layer);
    let b1 = create_node(&graph, &layer);
    let a2 = create_node(&graph, &layer);
    let c1 = create_node(&graph, &layer);

    set_layout_unit(&a1, &a1);
    set_layout_unit(&a2, &a1);
    set_layout_unit(&b1, &b1);
    set_layout_unit(&c1, &c1);

    let mut node_order = vec![a1, b1, a2, c1];
    let mut resolver = prepare_resolver(&node_order);
    resolver.process_constraints(&mut node_order);

    assert_non_overlapping_layout_units(&node_order);
}

fn create_graph_with_layer() -> (LGraphRef, LayerRef) {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(layer.clone());
    }
    (graph, layer)
}

fn create_node(graph: &LGraphRef, layer: &LayerRef) -> LNodeRef {
    let node = LNode::new(graph);
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_node_type(NodeType::Normal);
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn set_layout_unit(node: &LNodeRef, representative: &LNodeRef) {
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_property(
            InternalProperties::IN_LAYER_LAYOUT_UNIT,
            Some(representative.clone()),
        );
    }
}

fn prepare_resolver(nodes: &[LNodeRef]) -> ForsterConstraintResolver {
    let node_order = vec![nodes.to_vec()];
    let mut resolver = ForsterConstraintResolver::new(&node_order, false);
    resolver.init_at_layer_level(0, &node_order);
    for node_index in 0..nodes.len() {
        resolver.init_at_node_level(0, node_index, &node_order);
    }

    let barycenter_states = resolver.barycenter_states();
    if let Some(layer_states) = barycenter_states.first() {
        for (index, state) in layer_states.iter().enumerate() {
            if let Ok(mut state_guard) = state.lock() {
                state_guard.barycenter = Some(index as f64);
            }
        }
    }

    resolver
}

fn assert_successor_constraints_respected(nodes: &[LNodeRef]) {
    let mut encountered_nodes = HashSet::new();

    for node in nodes {
        let successors = node
            .lock()
            .ok()
            .and_then(|mut node_guard| {
                node_guard.get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
            })
            .unwrap_or_default();

        let has_violation = successors
            .iter()
            .any(|successor| encountered_nodes.contains(&node_ptr_id(successor)));
        assert!(!has_violation, "successor constraints are not respected");

        encountered_nodes.insert(node_ptr_id(node));
    }
}

fn assert_non_overlapping_layout_units(nodes: &[LNodeRef]) {
    let mut encountered_units = HashSet::new();
    let mut current_layout_unit: Option<usize> = None;

    for node in nodes {
        let layout_unit = node
            .lock()
            .ok()
            .and_then(|mut node_guard| {
                node_guard.get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT)
            })
            .map(|layout_unit| node_ptr_id(&layout_unit));

        if let Some(layout_unit) = layout_unit {
            if Some(layout_unit) != current_layout_unit {
                assert!(
                    !encountered_units.contains(&layout_unit),
                    "layout units overlap after constraint resolving"
                );

                if let Some(previous_unit) = current_layout_unit {
                    encountered_units.insert(previous_unit);
                }
                current_layout_unit = Some(layout_unit);
            }
        }
    }
}

fn node_ptr_id(node: &LNodeRef) -> usize {
    Arc::as_ptr(node) as usize
}
