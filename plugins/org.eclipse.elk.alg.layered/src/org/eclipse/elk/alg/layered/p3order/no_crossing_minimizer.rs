use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::LGraph;
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

static INTERMEDIATE_PROCESSING_CONFIGURATION: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P3NodeOrdering,
            Arc::new(IntermediateProcessorStrategy::LongEdgeSplitter),
        )
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::InLayerConstraintProcessor),
        )
        .after(LayeredPhases::P5EdgeRouting)
        .add(Arc::new(IntermediateProcessorStrategy::LongEdgeJoiner));
    config
});

pub struct NoCrossingMinimizer;

impl NoCrossingMinimizer {
    pub fn new() -> Self {
        NoCrossingMinimizer
    }
}

impl Default for NoCrossingMinimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for NoCrossingMinimizer {
    fn process(&mut self, _graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("No crossing minimization", 1.0);
        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        let mut config = LayoutProcessorConfiguration::create_from(
            &INTERMEDIATE_PROCESSING_CONFIGURATION,
        );
        config.add_before(
            LayeredPhases::P3NodeOrdering,
            Arc::new(IntermediateProcessorStrategy::PortListSorter),
        );
        Some(config)
    }
}
