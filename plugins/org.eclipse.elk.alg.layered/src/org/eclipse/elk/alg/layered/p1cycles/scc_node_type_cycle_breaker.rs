use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNodeRef};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;
use crate::org::eclipse::elk::alg::layered::p1cycles::group_model_order_calculator::GroupModelOrderCalculator;
use crate::org::eclipse::elk::alg::layered::p1cycles::scc_model_order_cycle_breaker::{
    constraint_model_order, contains_node, layout_processor_configuration,
    node_group_model_order_id, process_scc_model_order_cycle_breaking,
};
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

pub struct SccNodeTypeCycleBreaker;

impl SccNodeTypeCycleBreaker {
    pub fn new() -> Self {
        SccNodeTypeCycleBreaker
    }

    fn find_nodes(
        layered_graph: &mut LGraph,
        strongly_connected_components: &[Vec<LNodeRef>],
        offset: i32,
        big_offset: i32,
        rev_edges: &mut Vec<crate::org::eclipse::elk::alg::layered::graph::LEdgeRef>,
    ) {
        let preferred_source = layered_graph
            .get_property(LayeredOptions::GROUP_MODEL_ORDER_CB_PREFERRED_SOURCE_ID)
            .unwrap_or(0);
        let preferred_target = layered_graph
            .get_property(LayeredOptions::GROUP_MODEL_ORDER_CB_PREFERRED_TARGET_ID)
            .unwrap_or(0);

        for component in strongly_connected_components {
            if component.len() <= 1 {
                continue;
            }

            let mut min_node: Option<LNodeRef> = None;
            let mut max_node: Option<LNodeRef> = None;
            let mut min_model_order = i32::MAX;
            let mut max_model_order = i32::MIN;

            let mut calculator = GroupModelOrderCalculator::new();
            for node in component {
                let current = constraint_model_order(
                    layered_graph,
                    node,
                    &mut calculator,
                    offset,
                    big_offset,
                );
                if min_node.is_none() || current < min_model_order {
                    min_node = Some(node.clone());
                    min_model_order = current;
                }
                if max_node.is_none() || current > max_model_order {
                    max_node = Some(node.clone());
                    max_model_order = current;
                }
            }

            let (Some(min_node), Some(max_node)) = (min_node, max_node) else {
                continue;
            };

            if node_group_model_order_id(&min_node) == preferred_source {
                let incoming_edges = min_node
                    .lock_ok()
                    .map(|node_guard| node_guard.incoming_edges())
                    .unwrap_or_default();
                for edge in incoming_edges {
                    let source_node = edge
                        .lock_ok()
                        .and_then(|edge_guard| edge_guard.source())
                        .and_then(|source| {
                            source
                                .lock_ok()
                                .and_then(|source_guard| source_guard.node())
                        });
                    let Some(source_node) = source_node else {
                        continue;
                    };
                    if contains_node(component, &source_node) {
                        rev_edges.push(edge);
                    }
                }
                continue;
            }

            if node_group_model_order_id(&max_node) == preferred_target {
                let outgoing_edges = max_node
                    .lock_ok()
                    .map(|node_guard| node_guard.outgoing_edges())
                    .unwrap_or_default();
                for edge in outgoing_edges {
                    let source_node = edge
                        .lock_ok()
                        .and_then(|edge_guard| edge_guard.source())
                        .and_then(|source| {
                            source
                                .lock_ok()
                                .and_then(|source_guard| source_guard.node())
                        });
                    let Some(source_node) = source_node else {
                        continue;
                    };
                    if contains_node(component, &source_node) {
                        rev_edges.push(edge);
                    }
                }
                continue;
            }

            let min_in_degree = min_node
                .lock_ok()
                .map(|node_guard| node_guard.incoming_edges().len())
                .unwrap_or(0);
            let max_out_degree = max_node
                .lock_ok()
                .map(|node_guard| node_guard.outgoing_edges().len())
                .unwrap_or(0);

            if min_in_degree > max_out_degree {
                let incoming_edges = min_node
                    .lock_ok()
                    .map(|node_guard| node_guard.incoming_edges())
                    .unwrap_or_default();
                for edge in incoming_edges {
                    let source_node = edge
                        .lock_ok()
                        .and_then(|edge_guard| edge_guard.source())
                        .and_then(|source| {
                            source
                                .lock_ok()
                                .and_then(|source_guard| source_guard.node())
                        });
                    let Some(source_node) = source_node else {
                        continue;
                    };
                    if contains_node(component, &source_node) {
                        rev_edges.push(edge);
                    }
                }
            } else {
                let outgoing_edges = max_node
                    .lock_ok()
                    .map(|node_guard| node_guard.outgoing_edges())
                    .unwrap_or_default();
                for edge in outgoing_edges {
                    let target_node = edge
                        .lock_ok()
                        .and_then(|edge_guard| edge_guard.target())
                        .and_then(|target| {
                            target
                                .lock_ok()
                                .and_then(|target_guard| target_guard.node())
                        });
                    let Some(target_node) = target_node else {
                        continue;
                    };
                    if contains_node(component, &target_node) {
                        rev_edges.push(edge);
                    }
                }
            }
        }
    }
}

impl Default for SccNodeTypeCycleBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for SccNodeTypeCycleBreaker {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Model order cycle breaking", 1.0);

        process_scc_model_order_cycle_breaking(layered_graph, Self::find_nodes);

        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        Some(layout_processor_configuration())
    }
}
