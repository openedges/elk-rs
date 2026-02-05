use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::TGraphRef;
use crate::org::eclipse::elk::alg::mrtree::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::mrtree::tree_layout_phases::TreeLayoutPhases;

#[derive(Default)]
pub struct OrderBalance;

impl ILayoutPhase<TreeLayoutPhases, TGraphRef> for OrderBalance {
    fn process(&mut self, _graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Processor arrange node", 1.0);
        // TODO: Port full order balancing logic when this phase becomes selectable.
        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &TGraphRef,
    ) -> Option<LayoutProcessorConfiguration<TreeLayoutPhases, TGraphRef>> {
        let mut config = LayoutProcessorConfiguration::create();
        config
            .before(TreeLayoutPhases::P2NodeOrdering)
            .add(std::sync::Arc::new(IntermediateProcessorStrategy::RootProc))
            .add(std::sync::Arc::new(IntermediateProcessorStrategy::FanProc))
            .add(std::sync::Arc::new(IntermediateProcessorStrategy::NeighborsProc));
        Some(config)
    }
}
