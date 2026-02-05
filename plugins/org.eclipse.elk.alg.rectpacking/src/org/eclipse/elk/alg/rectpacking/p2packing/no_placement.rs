use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::options::{InternalProperties, RectPackingOptions};
use crate::org::eclipse::elk::alg::rectpacking::rect_packing_layout_phases::RectPackingLayoutPhases;
use crate::org::eclipse::elk::alg::rectpacking::util::DrawingUtil;

pub struct NoPlacement;

impl NoPlacement {
    pub fn new() -> Self {
        NoPlacement
    }
}

impl Default for NoPlacement {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<RectPackingLayoutPhases, ElkNodeRef> for NoPlacement {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("No Compaction", 1.0);
        let padding = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RectPackingOptions::PADDING)
        }
        .unwrap_or_default();

        let rectangles = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().iter().cloned().collect::<Vec<_>>()
        };

        let size = DrawingUtil::calculate_dimensions_from_nodes(&rectangles);
        let min_width = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(InternalProperties::MIN_WIDTH)
        }
        .unwrap_or(0.0);
        let min_height = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(InternalProperties::MIN_HEIGHT)
        }
        .unwrap_or(0.0);

        let width = size.x.max(min_width - (padding.left + padding.right));
        let height = size.y.max(min_height - (padding.top + padding.bottom));
        let additional_height = height - size.y;

        let mut graph_mut = graph.borrow_mut();
        let props = graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut();
        props.set_property(InternalProperties::ADDITIONAL_HEIGHT, Some(additional_height));
        props.set_property(InternalProperties::DRAWING_WIDTH, Some(width));
        props.set_property(InternalProperties::DRAWING_HEIGHT, Some(height + additional_height));
        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &ElkNodeRef,
    ) -> Option<LayoutProcessorConfiguration<RectPackingLayoutPhases, ElkNodeRef>> {
        None
    }
}
