use std::cmp::max;
use std::sync::{Arc, LazyLock};

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{
    GroupOrderStrategy, InternalProperties, LayeredOptions, PortType,
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

pub struct ModelOrderCycleBreaker;

impl ModelOrderCycleBreaker {
    pub fn new() -> Self {
        ModelOrderCycleBreaker
    }
}

impl Default for ModelOrderCycleBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for ModelOrderCycleBreaker {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Model order cycle breaking", 1.0);

        let mut rev_edges = Vec::new();
        let layerless_nodes = layered_graph.layerless_nodes().clone();
        let offset = max(
            layerless_nodes.len() as i32,
            layered_graph
                .get_property(InternalProperties::MAX_MODEL_ORDER_NODES)
                .unwrap_or(0),
        );
        let model_order_group_count = layered_graph
            .get_property(InternalProperties::CB_NUM_MODEL_ORDER_GROUPS)
            .unwrap_or(0)
            .max(1);
        let big_offset = offset * model_order_group_count;
        let enforce_group_model_order = layered_graph
            .get_property(LayeredOptions::GROUP_MODEL_ORDER_CB_GROUP_ORDER_STRATEGY)
            .unwrap_or(GroupOrderStrategy::OnlyWithinGroup)
            == GroupOrderStrategy::Enforced;

        for source in layerless_nodes {
            let mut calculator = GroupModelOrderCalculator::new();
            let model_order_source = if enforce_group_model_order {
                calculator.compute_constraint_group_model_order(&source, big_offset, offset)
            } else {
                calculator.compute_constraint_model_order(&source, offset)
            };

            let output_ports = source
                .lock()
                .ok()
                .map(|node_guard| node_guard.ports_by_type(PortType::Output))
                .unwrap_or_default();
            for port in output_ports {
                let outgoing_edges = port
                    .lock()
                    .ok()
                    .map(|port_guard| port_guard.outgoing_edges().clone())
                    .unwrap_or_default();
                for edge in outgoing_edges {
                    let target = edge
                        .lock()
                        .ok()
                        .and_then(|edge_guard| edge_guard.target())
                        .and_then(|target_port| {
                            target_port.lock().ok().and_then(|target_guard| target_guard.node())
                        });
                    let Some(target) = target else {
                        continue;
                    };

                    let model_order_target = if enforce_group_model_order {
                        calculator.compute_constraint_group_model_order(&target, big_offset, offset)
                    } else {
                        calculator.compute_constraint_model_order(&target, offset)
                    };
                    if model_order_target < model_order_source {
                        rev_edges.push(edge);
                    }
                }
            }
        }

        let dummy_graph = LGraph::new();
        for edge in rev_edges {
            LEdge::reverse(&edge, &dummy_graph, true);
            layered_graph.set_property(InternalProperties::CYCLIC, Some(true));
        }

        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        Some(LayoutProcessorConfiguration::create_from(
            &INTERMEDIATE_PROCESSING_CONFIGURATION,
        ))
    }
}
