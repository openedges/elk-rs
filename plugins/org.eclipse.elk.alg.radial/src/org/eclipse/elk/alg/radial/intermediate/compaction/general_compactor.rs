use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::options::RadialOptions;

#[derive(Default)]
pub struct GeneralCompactor;

impl ILayoutProcessor<ElkNodeRef> for GeneralCompactor {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("General Compactor", 1.0);
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "Before");
        }
        let compactor = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RadialOptions::COMPACTOR)
        }
        .unwrap_or_default();
        compactor.create().compact(graph);
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "After");
        }
    }
}
