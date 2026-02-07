use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::options::{InternalProperties, RectPackingOptions};
use crate::org::eclipse::elk::alg::rectpacking::p1widthapproximation::AreaApproximation;
use crate::org::eclipse::elk::alg::rectpacking::rect_packing_layout_phases::RectPackingLayoutPhases;
use crate::org::eclipse::elk::alg::rectpacking::util::DrawingUtil;

pub struct GreedyWidthApproximator;

impl GreedyWidthApproximator {
    pub fn new() -> Self {
        GreedyWidthApproximator
    }
}

impl Default for GreedyWidthApproximator {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<RectPackingLayoutPhases, ElkNodeRef> for GreedyWidthApproximator {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Greedy Width Approximator", 1.0);
        let aspect_ratio = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RectPackingOptions::ASPECT_RATIO)
        }
        .or_else(|| {
            LayoutMetaDataService::get_instance()
                .get_algorithm_data(RectPackingOptions::ALGORITHM_ID)
                .and_then(|algorithm| {
                    algorithm
                        .default_value_any(RectPackingOptions::ASPECT_RATIO.id())
                        .and_then(|value| value.downcast::<f64>().ok().map(|value| *value))
                })
        })
        .unwrap_or(1.3);
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
        let goal = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RectPackingOptions::WIDTH_APPROXIMATION_OPTIMIZATION_GOAL)
        }
        .unwrap_or_default();
        let last_place_shift = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RectPackingOptions::WIDTH_APPROXIMATION_LAST_PLACE_SHIFT)
        }
        .unwrap_or(true);
        let node_node_spacing = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RectPackingOptions::SPACING_NODE_NODE)
        }
        .unwrap_or(0.0);

        let rectangles = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().iter().cloned().collect::<Vec<_>>()
        };
        DrawingUtil::reset_coordinates(&rectangles);
        let first_it = AreaApproximation::new(aspect_ratio, goal, last_place_shift);
        let drawing = first_it.approx_bounding_box(&rectangles, node_node_spacing, &padding);

        {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .set_property(InternalProperties::TARGET_WIDTH, Some(drawing.drawing_width()));
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
