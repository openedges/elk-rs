use std::any::Any;

use crate::org::eclipse::elk::core::util::IElkProgressMonitor;

pub trait ILayoutProcessor<G>: Send + Any {
    fn process(&mut self, graph: &mut G, progress_monitor: &mut dyn IElkProgressMonitor);

    fn is_hierarchy_aware(&self) -> bool {
        false
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
