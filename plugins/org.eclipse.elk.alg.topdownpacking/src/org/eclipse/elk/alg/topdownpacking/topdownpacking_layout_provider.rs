use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::algorithm_assembler::{AlgorithmAssembler, SharedProcessor};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::topdown_layout_provider::ITopdownLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::topdownpacking::grid_elk_node::GridElkNode;
use crate::org::eclipse::elk::alg::topdownpacking::i_node_arranger::INodeArranger;
use crate::org::eclipse::elk::alg::topdownpacking::options::TopdownpackingOptions;
use crate::org::eclipse::elk::alg::topdownpacking::topdown_packing_phases::TopdownPackingPhases;

pub struct TopdownpackingLayoutProvider;

impl TopdownpackingLayoutProvider {
    pub fn new() -> Self {
        TopdownpackingLayoutProvider
    }

    pub fn assemble_algorithm(graph: &GridElkNode) -> Vec<SharedProcessor<GridElkNode>> {
        let mut algorithm_assembler: AlgorithmAssembler<TopdownPackingPhases, GridElkNode> =
            AlgorithmAssembler::create();

        let node_arrangement = graph
            .get_property(TopdownpackingOptions::NODE_ARRANGEMENT_STRATEGY)
            .unwrap_or_default();
        let whitespace_elimination = graph
            .get_property(TopdownpackingOptions::WHITESPACE_ELIMINATION_STRATEGY)
            .unwrap_or_default();

        let node_factory: Arc<dyn ILayoutPhaseFactory<TopdownPackingPhases, GridElkNode>> =
            Arc::new(node_arrangement);
        let whitespace_factory: Arc<
            dyn ILayoutPhaseFactory<TopdownPackingPhases, GridElkNode>,
        > = Arc::new(whitespace_elimination);

        algorithm_assembler.set_phase(
            TopdownPackingPhases::P1NodeArrangement,
            node_factory,
        );
        algorithm_assembler.set_phase(
            TopdownPackingPhases::P2WhitespaceElimination,
            whitespace_factory,
        );

        algorithm_assembler.build(graph)
    }
}

impl Default for TopdownpackingLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphLayoutEngine for TopdownpackingLayoutProvider {
    fn layout(
        &mut self,
        layout_graph: &ElkNodeRef,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        let mut wrapped_graph = GridElkNode::new(layout_graph.clone());
        let algorithm = Self::assemble_algorithm(&wrapped_graph);

        progress_monitor.begin("Topdown Packing", algorithm.len() as f32);

        for processor in &algorithm {
            let mut processor_guard = processor.lock().expect("processor lock");
            let mut sub = progress_monitor.sub_task(1.0);
            processor_guard.process(&mut wrapped_graph, sub.as_mut());
        }

        progress_monitor.done();
    }
}

impl ITopdownLayoutProvider for TopdownpackingLayoutProvider {
    fn get_predicted_graph_size(&self, graph: &ElkNodeRef) -> KVector {
        let wrapper = GridElkNode::new(graph.clone());
        let strategy = wrapper
            .get_property(TopdownpackingOptions::NODE_ARRANGEMENT_STRATEGY)
            .unwrap_or_default();
        let arranger: Box<dyn INodeArranger> = strategy.create_arranger();
        arranger.get_predicted_size(graph)
    }
}

impl AbstractLayoutProvider for TopdownpackingLayoutProvider {}
