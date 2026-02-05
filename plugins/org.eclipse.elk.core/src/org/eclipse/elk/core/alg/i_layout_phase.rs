use crate::org::eclipse::elk::core::util::{EnumSetType, IElkProgressMonitor};

use super::layout_processor_configuration::LayoutProcessorConfiguration;

pub trait ILayoutPhase<P: EnumSetType, G>: Send {
    fn process(&mut self, graph: &mut G, progress_monitor: &mut dyn IElkProgressMonitor);

    fn get_layout_processor_configuration(
        &self,
        graph: &G,
    ) -> Option<LayoutProcessorConfiguration<P, G>>;

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
