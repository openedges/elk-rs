use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkGraphFactory, ElkNode, ElkNodeRef};

use crate::org::eclipse::elk::alg::rectpacking::options::{InternalProperties, RectPackingOptions};
use crate::org::eclipse::elk::alg::rectpacking::p2packing::RowFillingAndCompaction;
use crate::org::eclipse::elk::alg::rectpacking::rect_packing_layout_phases::RectPackingLayoutPhases;

pub struct Compactor;

impl Compactor {
    pub fn new() -> Self {
        Compactor
    }
}

impl Default for Compactor {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<RectPackingLayoutPhases, ElkNodeRef> for Compactor {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Compaction", 1.0);
        let aspect_ratio = property(graph, RectPackingOptions::ASPECT_RATIO).unwrap_or(1.0);
        let node_node_spacing = property(graph, RectPackingOptions::SPACING_NODE_NODE).unwrap_or(0.0);
        let padding = property(graph, RectPackingOptions::PADDING).unwrap_or_default();

        let rectangles = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().iter().cloned().collect::<Vec<_>>()
        };

        let mut second_it = RowFillingAndCompaction::new(aspect_ratio, node_node_spacing);
        let mut drawing = second_it.start(&rectangles, progress_monitor, graph, &padding);
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "Compacted");
        }
        Self::copy_row_width_change_values(graph, &second_it);

        let mut iterations =
            property(graph, RectPackingOptions::PACKING_COMPACTION_ITERATIONS).unwrap_or(1);
        while iterations > 1 {
            let clone = Self::clone_graph(graph);
            let old_sm = drawing.scale_measure();
            Self::configure_second_iteration(graph, &clone, &drawing);

            second_it = RowFillingAndCompaction::new(aspect_ratio, node_node_spacing);
            let new_drawing = second_it.start(&rectangles, progress_monitor, &clone, &padding);

            if progress_monitor.is_logging_enabled() {
                progress_monitor.log_graph(&clone, &format!("Layouted clone {}", iterations));
            }

            let new_sm = new_drawing.scale_measure();
            if new_sm >= old_sm && new_sm == new_sm {
                let clone_children = {
                    let mut clone_mut = clone.borrow_mut();
                    clone_mut.children().iter().cloned().collect::<Vec<_>>()
                };
                let graph_children = {
                    let mut graph_mut = graph.borrow_mut();
                    graph_mut.children().iter().cloned().collect::<Vec<_>>()
                };
                for (clone_child, graph_child) in clone_children.iter().zip(graph_children.iter()) {
                    Self::copy_position(clone_child, graph_child);
                }
                Self::copy_row_width_change_values(graph, &second_it);
                drawing.set_drawing_width(new_drawing.drawing_width());
                drawing.set_drawing_height(new_drawing.drawing_height());
            }
            iterations -= 1;
        }

        set_property(graph, InternalProperties::DRAWING_HEIGHT, drawing.drawing_height());
        set_property(graph, InternalProperties::DRAWING_WIDTH, drawing.drawing_width());
        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &ElkNodeRef,
    ) -> Option<LayoutProcessorConfiguration<RectPackingLayoutPhases, ElkNodeRef>> {
        None
    }
}

impl Compactor {
    fn copy_row_width_change_values(graph: &ElkNodeRef, compaction: &RowFillingAndCompaction) {
        set_property(graph, InternalProperties::MIN_ROW_INCREASE, compaction.potential_row_width_increase_min);
        set_property(graph, InternalProperties::MAX_ROW_INCREASE, compaction.potential_row_width_increase_max);
        set_property(graph, InternalProperties::MIN_ROW_DECREASE, compaction.potential_row_width_decrease_min);
        set_property(graph, InternalProperties::MAX_ROW_DECREASE, compaction.potential_row_width_decrease_max);
    }

    fn configure_second_iteration(layout_graph: &ElkNodeRef, clone: &ElkNodeRef, drawing: &crate::org::eclipse::elk::alg::rectpacking::util::DrawingData) {
        let padding = property(layout_graph, RectPackingOptions::PADDING).unwrap_or_default();
        let aspect_ratio = property(layout_graph, RectPackingOptions::ASPECT_RATIO).unwrap_or(1.0);
        let min_row_increase = property(layout_graph, InternalProperties::MIN_ROW_INCREASE)
            .unwrap_or(f64::INFINITY);
        let min_row_decrease = property(layout_graph, InternalProperties::MIN_ROW_DECREASE)
            .unwrap_or(f64::INFINITY);
        let target_width = property(layout_graph, InternalProperties::TARGET_WIDTH).unwrap_or(0.0);
        let min_width = property(layout_graph, InternalProperties::MIN_WIDTH).unwrap_or(0.0);

        let drawing_ratio = (drawing.drawing_width() + padding.left + padding.right)
            / (drawing.drawing_height() + padding.top + padding.bottom);
        let children_len = {
            let mut graph_mut = layout_graph.borrow_mut();
            graph_mut.children().len()
        };

        if children_len > 1
            && min_row_increase != f64::INFINITY
            && drawing_ratio < aspect_ratio
        {
            set_property(
                clone,
                InternalProperties::TARGET_WIDTH,
                target_width + min_row_increase,
            );
        } else if children_len > 1
            && min_row_decrease != f64::INFINITY
            && drawing_ratio > aspect_ratio
        {
            let new_width = (target_width - min_row_decrease).max(min_width);
            set_property(clone, InternalProperties::TARGET_WIDTH, new_width);
        }
    }

    fn clone_graph(node: &ElkNodeRef) -> ElkNodeRef {
        let clone = ElkGraphFactory::instance().create_elk_node();
        {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            let properties = shape.graph_element().properties().clone();
            let identifier = shape.graph_element().identifier().map(|value| value.to_string());
            let (width, height, x, y) = (shape.width(), shape.height(), shape.x(), shape.y());

            let mut clone_mut = clone.borrow_mut();
            let clone_shape = clone_mut.connectable().shape();
            clone_shape.set_dimensions(width, height);
            clone_shape.set_location(x, y);
            clone_shape
                .graph_element()
                .properties_mut()
                .copy_properties(&properties);
            clone_shape.graph_element().set_identifier(identifier);
        }

        let children = {
            let mut node_mut = node.borrow_mut();
            node_mut.children().iter().cloned().collect::<Vec<_>>()
        };
        for child in children {
            let new_child = ElkGraphFactory::instance().create_elk_node();
            ElkNode::set_parent(&new_child, Some(clone.clone()));

            let mut child_mut = child.borrow_mut();
            let child_shape = child_mut.connectable().shape();
            let properties = child_shape.graph_element().properties().clone();
            let identifier = child_shape.graph_element().identifier().map(|value| value.to_string());
            let (width, height, x, y) = (child_shape.width(), child_shape.height(), child_shape.x(), child_shape.y());
            drop(child_mut);

            let mut new_child_mut = new_child.borrow_mut();
            let new_shape = new_child_mut.connectable().shape();
            new_shape.set_dimensions(width, height);
            new_shape.set_location(x, y);
            new_shape
                .graph_element()
                .properties_mut()
                .copy_properties(&properties);
            new_shape.graph_element().set_identifier(identifier);
        }
        clone
    }

    fn copy_position(source: &ElkNodeRef, target: &ElkNodeRef) {
        let (width, height, x, y) = {
            let mut source_mut = source.borrow_mut();
            let shape = source_mut.connectable().shape();
            (shape.width(), shape.height(), shape.x(), shape.y())
        };
        {
            let mut target_mut = target.borrow_mut();
            let shape = target_mut.connectable().shape();
            shape.set_dimensions(width, height);
            shape.set_location(x, y);
        }

        let source_children = {
            let mut source_mut = source.borrow_mut();
            source_mut.children().iter().cloned().collect::<Vec<_>>()
        };
        let target_children = {
            let mut target_mut = target.borrow_mut();
            target_mut.children().iter().cloned().collect::<Vec<_>>()
        };
        for (src_child, tgt_child) in source_children.iter().zip(target_children.iter()) {
            Self::copy_position(src_child, tgt_child);
        }
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
