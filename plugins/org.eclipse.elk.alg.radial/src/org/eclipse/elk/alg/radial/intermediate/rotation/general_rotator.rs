use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::rotation::{AngleRotation, IRadialRotator};

#[derive(Default)]
pub struct GeneralRotator;

impl ILayoutProcessor<ElkNodeRef> for GeneralRotator {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("General Rotator", 1.0);
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "Before");
        }
        let mut rotator = AngleRotation;
        rotator.rotate(graph);
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "After");
        }
    }
}
