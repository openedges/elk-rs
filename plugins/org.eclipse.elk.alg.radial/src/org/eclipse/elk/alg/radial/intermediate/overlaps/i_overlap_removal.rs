use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub trait IOverlapRemoval {
    fn remove_overlaps(&mut self, graph: &ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor);
}
