use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::algorithm_assembler::{
    AlgorithmAssembler, SharedProcessor,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::spore::graph::Graph;
use crate::org::eclipse::elk::alg::spore::options::StructureExtractionStrategy;
use crate::org::eclipse::elk::alg::spore::spore_phases::SPOrEPhases;

pub struct ShrinkTree {
    algorithm_assembler: AlgorithmAssembler<SPOrEPhases, Graph>,
    algorithm: Vec<SharedProcessor<Graph>>,
}

impl ShrinkTree {
    pub fn new() -> Self {
        ShrinkTree {
            algorithm_assembler: AlgorithmAssembler::create(),
            algorithm: Vec::new(),
        }
    }

    pub fn shrink(&mut self, graph: &mut Graph, progress_monitor: &mut dyn IElkProgressMonitor) {
        self.algorithm_assembler.reset();
        self.algorithm_assembler
            .set_phase(
                SPOrEPhases::P1Structure,
                Arc::new(StructureExtractionStrategy::DelaunayTriangulation),
            )
            .set_phase(
                SPOrEPhases::P2ProcessingOrder,
                Arc::new(graph.tree_construction_strategy),
            )
            .set_phase(
                SPOrEPhases::P3Execution,
                Arc::new(graph.compaction_strategy),
            );

        self.algorithm = self.algorithm_assembler.build(graph);

        let total = self.algorithm.len().max(1) as f32;
        progress_monitor.begin("Compaction by shrinking a tree", total);

        if graph.vertices.len() > 1 {
            for processor in &self.algorithm {
                if progress_monitor.is_canceled() {
                    return;
                }
                if let Ok(mut processor_guard) = processor.lock() {
                    let mut sub = progress_monitor.sub_task(1.0);
                    processor_guard.process(graph, sub.as_mut());
                }
            }
        }

        graph.sync_vertices_from_tree();
        progress_monitor.done();
    }
}

impl Default for ShrinkTree {
    fn default() -> Self {
        Self::new()
    }
}
