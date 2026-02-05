use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::LGraph;

#[derive(Default)]
pub struct CompoundGraphPreprocessor;

impl CompoundGraphPreprocessor {
    pub fn new() -> Self {
        CompoundGraphPreprocessor
    }
}

impl ILayoutProcessor<LGraph> for CompoundGraphPreprocessor {
    fn process(&mut self, _graph: &mut LGraph, _progress_monitor: &mut dyn IElkProgressMonitor) {}
}

#[derive(Default)]
pub struct CompoundGraphPostprocessor;

impl CompoundGraphPostprocessor {
    pub fn new() -> Self {
        CompoundGraphPostprocessor
    }
}

impl ILayoutProcessor<LGraph> for CompoundGraphPostprocessor {
    fn process(&mut self, _graph: &mut LGraph, _progress_monitor: &mut dyn IElkProgressMonitor) {}
}
