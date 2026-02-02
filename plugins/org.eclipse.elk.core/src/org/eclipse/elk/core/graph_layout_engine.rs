use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::core::util::IElkProgressMonitor;

pub trait IGraphLayoutEngine: Send {
    fn layout(&mut self, layout_graph: &ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor);
}
