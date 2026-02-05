use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::libavoid::server::LibavoidServer;

pub struct LibavoidServerCommunicator;

impl LibavoidServerCommunicator {
    pub fn new() -> Self {
        LibavoidServerCommunicator
    }

    pub fn request_layout(
        &mut self,
        _layout_node: &ElkNodeRef,
        progress_monitor: &mut dyn IElkProgressMonitor,
        _server: &mut LibavoidServer,
    ) {
        progress_monitor.begin("Libavoid Layout", 1.0);
        progress_monitor.done();
    }
}

impl Default for LibavoidServerCommunicator {
    fn default() -> Self {
        Self::new()
    }
}
