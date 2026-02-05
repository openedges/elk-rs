use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::unsupported_configuration::UnsupportedConfigurationException;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::options::{InternalProperties, RectPackingOptions};
use crate::org::eclipse::elk::alg::rectpacking::p3whitespaceelimination::RectangleExpansion;
use crate::org::eclipse::elk::alg::rectpacking::rect_packing_layout_phases::RectPackingLayoutPhases;
use crate::org::eclipse::elk::alg::rectpacking::util::rows_storage;

pub struct EqualWhitespaceEliminator;

pub struct NoopWhitespaceEliminator;

impl EqualWhitespaceEliminator {
    pub fn new() -> Self {
        EqualWhitespaceEliminator
    }

    pub fn noop() -> NoopWhitespaceEliminator {
        NoopWhitespaceEliminator
    }
}

impl Default for EqualWhitespaceEliminator {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<RectPackingLayoutPhases, ElkNodeRef> for EqualWhitespaceEliminator {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Equal Whitespace Eliminator", 1.0);
        let rows_key = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(InternalProperties::ROWS)
        };

        if let Some(rows_key) = rows_key {
            if let Some(rows) = rows_storage::take_rows(rows_key) {
                let drawing_width = property(graph, InternalProperties::DRAWING_WIDTH).unwrap_or(0.0);
                let additional_height =
                    property(graph, InternalProperties::ADDITIONAL_HEIGHT).unwrap_or(0.0);
                let node_node_spacing =
                    property(graph, RectPackingOptions::SPACING_NODE_NODE).unwrap_or(0.0);
                RectangleExpansion::expand(&rows, drawing_width, additional_height, node_node_spacing);
            } else {
                panic!(
                    "{}",
                    UnsupportedConfigurationException::new("The graph does not contain rows.")
                );
            }
        } else {
            panic!(
                "{}",
                UnsupportedConfigurationException::new("The graph does not contain rows.")
            );
        }
        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &ElkNodeRef,
    ) -> Option<LayoutProcessorConfiguration<RectPackingLayoutPhases, ElkNodeRef>> {
        None
    }
}

impl ILayoutPhase<RectPackingLayoutPhases, ElkNodeRef> for NoopWhitespaceEliminator {
    fn process(&mut self, _graph: &mut ElkNodeRef, _progress_monitor: &mut dyn IElkProgressMonitor) {}

    fn get_layout_processor_configuration(
        &self,
        _graph: &ElkNodeRef,
    ) -> Option<LayoutProcessorConfiguration<RectPackingLayoutPhases, ElkNodeRef>> {
        None
    }
}

fn property<T: Clone + Send + Sync + 'static>(
    graph: &ElkNodeRef,
    property: &'static org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<T>,
) -> Option<T> {
    let mut graph_mut = graph.borrow_mut();
    graph_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
}
