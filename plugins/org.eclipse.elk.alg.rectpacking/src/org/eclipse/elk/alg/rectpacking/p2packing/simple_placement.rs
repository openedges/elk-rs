use std::cell::RefCell;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::options::{InternalProperties, RectPackingOptions};
use crate::org::eclipse::elk::alg::rectpacking::p2packing::InitialPlacement;
use crate::org::eclipse::elk::alg::rectpacking::rect_packing_layout_phases::RectPackingLayoutPhases;
use crate::org::eclipse::elk::alg::rectpacking::util::{BlockStack, DrawingUtil};
use crate::org::eclipse::elk::alg::rectpacking::util::rows_storage;

pub struct SimplePlacement;

impl SimplePlacement {
    pub fn new() -> Self {
        SimplePlacement
    }
}

impl Default for SimplePlacement {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<RectPackingLayoutPhases, ElkNodeRef> for SimplePlacement {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("No Compaction", 1.0);

        let target_width = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(InternalProperties::TARGET_WIDTH)
        }
        .unwrap_or(0.0);
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
        DrawingUtil::reset_coordinates(&rectangles);

        let rows = InitialPlacement::place(&rectangles, target_width, node_node_spacing);

        let size = if rows.is_empty() {
            DrawingUtil::calculate_dimensions_from_nodes(&rectangles)
        } else {
            for row in &rows {
                let blocks = { row.borrow().children().clone() };
                for block in blocks {
                    let stack_ref = Rc::new(RefCell::new(BlockStack::new(
                        block.borrow().x(),
                        block.borrow().y(),
                        node_node_spacing,
                    )));
                    BlockStack::add_block(&stack_ref, block);
                    row.borrow_mut().add_stack(stack_ref);
                }
            }
            DrawingUtil::calculate_dimensions(&rows, node_node_spacing)
        };

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
        let rows_key = rows_storage::store_rows(graph, rows);
        props.set_property(InternalProperties::ROWS, Some(rows_key));

        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &ElkNodeRef,
    ) -> Option<LayoutProcessorConfiguration<RectPackingLayoutPhases, ElkNodeRef>> {
        None
    }
}
