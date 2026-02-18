use std::sync::Arc;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::NodeMicroLayout;
use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::algorithm_assembler::{
    AlgorithmAssembler, SharedProcessor,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    BasicProgressMonitor, BoxLayoutProvider, ElkUtil, IElkProgressMonitor,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::rectpacking::options::{InternalProperties, RectPackingOptions};
use crate::org::eclipse::elk::alg::rectpacking::rect_packing_layout_phases::RectPackingLayoutPhases;

pub struct RectPackingLayoutProvider;

impl RectPackingLayoutProvider {
    pub fn new() -> Self {
        RectPackingLayoutProvider
    }

    fn assemble_algorithm(
        &mut self,
        layout_graph: &ElkNodeRef,
    ) -> Vec<SharedProcessor<ElkNodeRef>> {
        let mut algorithm_assembler: AlgorithmAssembler<RectPackingLayoutPhases, ElkNodeRef> =
            AlgorithmAssembler::create();

        let width_strategy = property(
            layout_graph,
            RectPackingOptions::WIDTH_APPROXIMATION_STRATEGY,
        )
        .unwrap_or_default();
        let packing_strategy =
            property(layout_graph, RectPackingOptions::PACKING_STRATEGY).unwrap_or_default();
        let whitespace_strategy = property(
            layout_graph,
            RectPackingOptions::WHITE_SPACE_ELIMINATION_STRATEGY,
        )
        .unwrap_or_default();

        let width_factory: Arc<dyn ILayoutPhaseFactory<RectPackingLayoutPhases, ElkNodeRef>> =
            Arc::new(width_strategy);
        let packing_factory: Arc<dyn ILayoutPhaseFactory<RectPackingLayoutPhases, ElkNodeRef>> =
            Arc::new(packing_strategy);
        let whitespace_factory: Arc<dyn ILayoutPhaseFactory<RectPackingLayoutPhases, ElkNodeRef>> =
            Arc::new(whitespace_strategy);

        algorithm_assembler.set_phase(RectPackingLayoutPhases::P1WidthApproximation, width_factory);
        algorithm_assembler.set_phase(RectPackingLayoutPhases::P2Packing, packing_factory);
        algorithm_assembler.set_phase(
            RectPackingLayoutPhases::P3WhitespaceElimination,
            whitespace_factory,
        );

        let config = self.get_phase_independent_layout_processor_configuration(layout_graph);
        algorithm_assembler.add_processor_configuration(&config);

        algorithm_assembler.build(layout_graph)
    }

    fn get_phase_independent_layout_processor_configuration(
        &self,
        layout_graph: &ElkNodeRef,
    ) -> LayoutProcessorConfiguration<RectPackingLayoutPhases, ElkNodeRef> {
        let mut configuration = LayoutProcessorConfiguration::create();

        configuration.add_before(
            RectPackingLayoutPhases::P1WidthApproximation,
            Arc::new(IntermediateProcessorStrategy::MinSizePreProcessor),
        );
        configuration.add_before(
            RectPackingLayoutPhases::P2Packing,
            Arc::new(IntermediateProcessorStrategy::MinSizePostProcessor),
        );

        if property(layout_graph, RectPackingOptions::ORDER_BY_SIZE).unwrap_or(false) {
            configuration.add_before(
                RectPackingLayoutPhases::P1WidthApproximation,
                Arc::new(IntermediateProcessorStrategy::NodeSizeReorderer),
            );
        }

        if property(layout_graph, RectPackingOptions::INTERACTIVE).unwrap_or(false) {
            configuration.add_before(
                RectPackingLayoutPhases::P1WidthApproximation,
                Arc::new(IntermediateProcessorStrategy::InteractiveNodeReorderer),
            );
        }

        configuration
    }
}

impl Default for RectPackingLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphLayoutEngine for RectPackingLayoutProvider {
    fn layout(
        &mut self,
        layout_graph: &ElkNodeRef,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        progress_monitor.begin("Rectangle Packing", 1.0);

        apply_algorithm_defaults(layout_graph);

        let padding = property(layout_graph, RectPackingOptions::PADDING).unwrap_or_default();
        let fixed_graph_size =
            property(layout_graph, RectPackingOptions::NODE_SIZE_FIXED_GRAPH_SIZE).unwrap_or(false);
        let node_node_spacing =
            property(layout_graph, RectPackingOptions::SPACING_NODE_NODE).unwrap_or(0.0);
        let try_box = property(layout_graph, RectPackingOptions::TRYBOX).unwrap_or(false);

        let rectangles = {
            let mut graph_mut = layout_graph.borrow_mut();
            graph_mut.children().iter().cloned().collect::<Vec<_>>()
        };

        if !property(layout_graph, RectPackingOptions::OMIT_NODE_MICRO_LAYOUT).unwrap_or(false) {
            NodeMicroLayout::for_graph(layout_graph.clone()).execute();
        }

        let mut stackable = false;
        if try_box && rectangles.len() >= 3 {
            let mut region2 = rectangles[0].clone();
            let mut region3 = rectangles[1].clone();
            let mut counter = 0usize;
            while counter + 2 < rectangles.len() {
                let region1 = region2;
                region2 = region3;
                region3 = rectangles[counter + 2].clone();
                let region1_height = node_height(&region1);
                let region2_height = node_height(&region2);
                let region3_height = node_height(&region3);
                if region1_height >= region2_height + region3_height + node_node_spacing
                    || region3_height >= region1_height + region2_height + node_node_spacing
                {
                    stackable = true;
                    break;
                }
                counter += 1;
            }
        } else {
            stackable = true;
        }

        if !stackable {
            let mut priority = rectangles.len() as i32;
            for rect in &rectangles {
                set_property(rect, CoreOptions::PRIORITY, priority);
                priority -= 1;
            }
            let mut box_layout = BoxLayoutProvider::new();
            let mut pm = BasicProgressMonitor::new();
            box_layout.layout(layout_graph, &mut pm);
            progress_monitor.done();
            return;
        }

        let algorithm = self.assemble_algorithm(layout_graph);
        let monitor_progress = if algorithm.is_empty() {
            1.0
        } else {
            1.0 / algorithm.len() as f32
        };

        let mut graph_ref = layout_graph.clone();
        let mut slot_index = 0usize;
        for processor in &algorithm {
            if progress_monitor.is_canceled() {
                return;
            }
            if progress_monitor.is_logging_enabled() {
                progress_monitor
                    .log_graph(layout_graph, &format!("{}-Before processor", slot_index));
            }
            let mut sub = progress_monitor.sub_task(monitor_progress);
            let mut processor_guard = processor.lock().expect("processor lock");
            processor_guard.process(&mut graph_ref, sub.as_mut());
            slot_index += 1;
        }

        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(layout_graph, &format!("{}-Finished", slot_index));
        }

        let mut real_width: f64 = 0.0;
        let mut real_height: f64 = 0.0;
        for rect in &rectangles {
            let (x, y, width, height) = node_bounds(rect);
            real_width = real_width.max(x + width);
            real_height = real_height.max(y + height);
        }

        let drawing_width =
            property(layout_graph, InternalProperties::DRAWING_WIDTH).unwrap_or(real_width);
        let drawing_height =
            property(layout_graph, InternalProperties::DRAWING_HEIGHT).unwrap_or(real_height);

        ElkUtil::translate((
            layout_graph,
            &KVector::with_values(drawing_width, drawing_height),
            &KVector::with_values(real_width, real_height),
        ));

        apply_padding(&rectangles, &padding);

        if !fixed_graph_size {
            ElkUtil::resize_node_with(
                layout_graph,
                drawing_width + padding.left + padding.right,
                drawing_height + padding.top + padding.bottom,
                false,
                true,
            );
        }

        if !property(layout_graph, RectPackingOptions::OMIT_NODE_MICRO_LAYOUT).unwrap_or(false) {
            NodeMicroLayout::for_graph(layout_graph.clone()).execute();
        }

        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(layout_graph, "Output");
        }
        progress_monitor.done();
    }
}

impl AbstractLayoutProvider for RectPackingLayoutProvider {}

fn apply_algorithm_defaults(graph: &ElkNodeRef) {
    let mut graph_mut = graph.borrow_mut();
    let props = graph_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    if !props.has_property_id(CoreOptions::PADDING.id()) {
        props.set_property(CoreOptions::PADDING, Some(ElkPadding::with_any(15.0)));
    }
    if !props.has_property_id(CoreOptions::SPACING_NODE_NODE.id()) {
        props.set_property(CoreOptions::SPACING_NODE_NODE, Some(15.0_f64));
    }
    if !props.has_property_id(CoreOptions::ASPECT_RATIO.id()) {
        props.set_property(CoreOptions::ASPECT_RATIO, Some(1.3_f64));
    }
}

fn apply_padding(
    rectangles: &[ElkNodeRef],
    padding: &org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding,
) {
    for rect in rectangles {
        let (x, y) = {
            let mut rect_mut = rect.borrow_mut();
            let shape = rect_mut.connectable().shape();
            (shape.x(), shape.y())
        };
        let mut rect_mut = rect.borrow_mut();
        rect_mut
            .connectable()
            .shape()
            .set_location(x + padding.left, y + padding.top);
    }
}

fn node_bounds(node: &ElkNodeRef) -> (f64, f64, f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}

fn node_height(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().height()
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
