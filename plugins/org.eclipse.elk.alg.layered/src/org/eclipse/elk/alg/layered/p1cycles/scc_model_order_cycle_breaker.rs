use std::cmp::max;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LNodeRef, NodeRefKey, Tarjan};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{
    GroupOrderStrategy, InternalProperties, LayeredOptions,
};
use crate::org::eclipse::elk::alg::layered::p1cycles::group_model_order_calculator::GroupModelOrderCalculator;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

static INTERMEDIATE_PROCESSING_CONFIGURATION: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config.add_after(
        LayeredPhases::P5EdgeRouting,
        Arc::new(IntermediateProcessorStrategy::ReversedEdgeRestorer),
    );
    config
});

pub(crate) fn layout_processor_configuration() -> LayoutProcessorConfiguration<LayeredPhases, LGraph>
{
    LayoutProcessorConfiguration::create_from(&INTERMEDIATE_PROCESSING_CONFIGURATION)
}

#[allow(clippy::mutable_key_type)]
pub(crate) fn process_scc_model_order_cycle_breaking<F>(
    layered_graph: &mut LGraph,
    mut find_nodes: F,
) where
    F: FnMut(
        &mut LGraph,
        &[Vec<LNodeRef>],
        i32,
        i32,
        &mut Vec<crate::org::eclipse::elk::alg::layered::graph::LEdgeRef>,
    ),
{
    let mut strongly_connected_components: Vec<Vec<LNodeRef>> = Vec::new();
    let mut node_to_scc_id: BTreeMap<NodeRefKey, usize> = BTreeMap::new();
    let mut rev_edges: Vec<crate::org::eclipse::elk::alg::layered::graph::LEdgeRef> = Vec::new();

    let offset = max(
        layered_graph.layerless_nodes().len() as i32,
        layered_graph
            .get_property(InternalProperties::MAX_MODEL_ORDER_NODES)
            .unwrap_or(0),
    );
    let model_order_group_count = layered_graph
        .get_property(InternalProperties::CB_NUM_MODEL_ORDER_GROUPS)
        .unwrap_or(0);
    let big_offset = offset * model_order_group_count;

    let tarjan_graph = LGraph::new();
    {
        let mut graph_guard = tarjan_graph.lock();
        graph_guard
            .layerless_nodes_mut()
            .extend(layered_graph.layerless_nodes().iter().cloned());
    }

    loop {
        {
            let mut tarjan = Tarjan::new(
                &rev_edges,
                &mut strongly_connected_components,
                &mut node_to_scc_id,
            );
            tarjan.reset_tarjan(&tarjan_graph);
            tarjan.tarjan(&tarjan_graph);
        }

        if strongly_connected_components.is_empty() {
            break;
        }

        find_nodes(
            layered_graph,
            &strongly_connected_components,
            offset,
            big_offset,
            &mut rev_edges,
        );

        let dummy_graph = LGraph::new();
        for edge in &rev_edges {
            LEdge::reverse(edge, &dummy_graph, false);
            increment_source_layer_id(edge);
            layered_graph.set_property(InternalProperties::CYCLIC, Some(true));
        }

        strongly_connected_components.clear();
        node_to_scc_id.clear();
        rev_edges.clear();
    }
}

pub(crate) fn constraint_model_order(
    graph: &mut LGraph,
    node: &LNodeRef,
    calculator: &mut GroupModelOrderCalculator,
    offset: i32,
    big_offset: i32,
) -> i32 {
    let enforce_group_model_order = graph
        .get_property(LayeredOptions::GROUP_MODEL_ORDER_CB_GROUP_ORDER_STRATEGY)
        .unwrap_or(GroupOrderStrategy::OnlyWithinGroup)
        == GroupOrderStrategy::Enforced;
    if enforce_group_model_order {
        calculator.compute_constraint_group_model_order(node, big_offset, offset)
    } else {
        calculator.compute_constraint_model_order(node, offset)
    }
}

pub(crate) fn node_group_model_order_id(node: &LNodeRef) -> i32 {
    node.lock_ok()
        .and_then(|mut node_guard| {
            node_guard.get_property(LayeredOptions::GROUP_MODEL_ORDER_CYCLE_BREAKING_ID)
        })
        .unwrap_or(0)
}

pub(crate) fn contains_node(component: &[LNodeRef], node: &LNodeRef) -> bool {
    component
        .iter()
        .any(|candidate| Arc::ptr_eq(candidate, node))
}

fn increment_source_layer_id(edge: &crate::org::eclipse::elk::alg::layered::graph::LEdgeRef) {
    let source_node = edge
        .lock().source()
        .and_then(|source_port| {
            source_port
                .lock().node()
        });
    let Some(source_node) = source_node else {
        return;
    };

    {
        let mut source_guard = source_node.lock();
        let current = source_guard
            .get_property(LayeredOptions::LAYERING_LAYER_ID)
            .unwrap_or(-1);
        source_guard.set_property(LayeredOptions::LAYERING_LAYER_ID, Some(current + 1));
    };
}
