use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BoxLayoutProvider, IElkProgressMonitor};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub struct Draw2DLayoutProvider {
    fallback: BoxLayoutProvider,
}

impl Draw2DLayoutProvider {
    pub fn new() -> Self {
        Draw2DLayoutProvider {
            fallback: BoxLayoutProvider::new(),
        }
    }
}

impl Default for Draw2DLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphLayoutEngine for Draw2DLayoutProvider {
    fn layout(&mut self, layout_graph: &ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        // Draw2D is not ported yet; keep API parity with a deterministic fallback provider.
        self.fallback.layout(layout_graph, progress_monitor);
    }
}

impl AbstractLayoutProvider for Draw2DLayoutProvider {}
