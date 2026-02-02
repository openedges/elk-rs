use crate::org::eclipse::elk::core::util::IElkProgressMonitor;

pub trait ILayoutProcessor<G> {
    fn process(&mut self, graph: &mut G, progress_monitor: &mut dyn IElkProgressMonitor);
}
