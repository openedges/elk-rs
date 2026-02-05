use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::options::{InternalProperties, RectPackingOptions};
use crate::org::eclipse::elk::alg::rectpacking::p3whitespaceelimination::EqualWhitespaceEliminator;

pub struct ToAspectratioNodeExpander;

impl ToAspectratioNodeExpander {
    pub fn new() -> Self {
        ToAspectratioNodeExpander
    }
}

impl Default for ToAspectratioNodeExpander {
    fn default() -> Self {
        Self::new()
    }
}

impl ToAspectratioNodeExpander {
    fn run(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("To Aspect Ratio Whitesapce Eliminator", 1.0);
        let mut width = property(graph, InternalProperties::DRAWING_WIDTH).unwrap_or(0.0);
        let mut height = property(graph, InternalProperties::DRAWING_HEIGHT).unwrap_or(0.0);
        let desired_aspect_ratio = property(graph, RectPackingOptions::ASPECT_RATIO).unwrap_or(1.0);
        let mut additional_height = property(graph, InternalProperties::ADDITIONAL_HEIGHT).unwrap_or(0.0);
        let aspect_ratio = if height.abs() > f64::EPSILON { width / height } else { 0.0 };
        if aspect_ratio < desired_aspect_ratio {
            width = height * desired_aspect_ratio;
            set_property(graph, InternalProperties::DRAWING_WIDTH, width);
        } else {
            additional_height += (width / desired_aspect_ratio) - height;
            height += additional_height;
            set_property(graph, InternalProperties::ADDITIONAL_HEIGHT, additional_height);
            set_property(graph, InternalProperties::DRAWING_HEIGHT, height);
        }
        let mut eliminator = EqualWhitespaceEliminator::new();
        eliminator.process(graph, progress_monitor);
        progress_monitor.done();
    }
}

impl org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase<
    crate::org::eclipse::elk::alg::rectpacking::rect_packing_layout_phases::RectPackingLayoutPhases,
    ElkNodeRef,
> for ToAspectratioNodeExpander {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        self.run(graph, progress_monitor);
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &ElkNodeRef,
    ) -> Option<
        org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration<
            crate::org::eclipse::elk::alg::rectpacking::rect_packing_layout_phases::RectPackingLayoutPhases,
            ElkNodeRef,
        >,
    > {
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

fn set_property<T: Clone + Send + Sync + 'static>(
    graph: &ElkNodeRef,
    property: &'static org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<T>,
    value: T,
) {
    let mut graph_mut = graph.borrow_mut();
    graph_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}
