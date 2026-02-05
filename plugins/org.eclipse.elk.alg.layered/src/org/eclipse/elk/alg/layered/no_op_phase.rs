use std::marker::PhantomData;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSetType, IElkProgressMonitor};

pub struct NoOpPhase<P, G> {
    _phantom: PhantomData<(P, G)>,
}

impl<P, G> NoOpPhase<P, G> {
    pub fn new() -> Self {
        NoOpPhase {
            _phantom: PhantomData,
        }
    }
}

impl<P, G> Default for NoOpPhase<P, G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P, G> ILayoutPhase<P, G> for NoOpPhase<P, G>
where
    P: EnumSetType + Send,
    G: Send,
{
    fn process(&mut self, _graph: &mut G, _progress_monitor: &mut dyn IElkProgressMonitor) {}

    fn get_layout_processor_configuration(
        &self,
        _graph: &G,
    ) -> Option<LayoutProcessorConfiguration<P, G>> {
        None
    }
}
