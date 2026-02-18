use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::options::{InternalProperties, RectPackingOptions};
use crate::org::eclipse::elk::alg::rectpacking::p2packing::{Compaction, InitialPlacement};
use crate::org::eclipse::elk::alg::rectpacking::util::rows_storage;
use crate::org::eclipse::elk::alg::rectpacking::util::{
    DrawingData, DrawingDataDescriptor, DrawingUtil, RectRowRef,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;

pub struct RowFillingAndCompaction {
    aspect_ratio: f64,
    node_node_spacing: f64,
    pub potential_row_width_decrease_min: f64,
    pub potential_row_width_decrease_max: f64,
    pub potential_row_width_increase_min: f64,
    pub potential_row_width_increase_max: f64,
}

impl RowFillingAndCompaction {
    pub fn new(aspect_ratio: f64, node_node_spacing: f64) -> Self {
        RowFillingAndCompaction {
            aspect_ratio,
            node_node_spacing,
            potential_row_width_decrease_min: f64::INFINITY,
            potential_row_width_decrease_max: 0.0,
            potential_row_width_increase_min: f64::INFINITY,
            potential_row_width_increase_max: 0.0,
        }
    }

    pub fn start(
        &mut self,
        _rectangles: &[ElkNodeRef],
        progress_monitor: &mut dyn IElkProgressMonitor,
        layout_graph: &ElkNodeRef,
        padding: &ElkPadding,
    ) -> DrawingData {
        let target_width = property(layout_graph, InternalProperties::TARGET_WIDTH).unwrap_or(0.0);
        let min_width = property(layout_graph, InternalProperties::MIN_WIDTH).unwrap_or(0.0);
        let min_height = property(layout_graph, InternalProperties::MIN_HEIGHT).unwrap_or(0.0);

        let rectangles = {
            let mut graph_mut = layout_graph.borrow_mut();
            graph_mut.children().iter().cloned().collect::<Vec<_>>()
        };
        DrawingUtil::reset_coordinates(&rectangles);

        let mut rows = InitialPlacement::place(&rectangles, target_width, self.node_node_spacing);

        let row_height_reevaluation = property(
            layout_graph,
            RectPackingOptions::PACKING_COMPACTION_ROW_HEIGHT_REEVALUATION,
        )
        .unwrap_or(false);

        let mut row_idx = 0usize;
        while row_idx < rows.len() {
            let current_row = rows[row_idx].clone();
            if row_idx != 0 {
                let previous_row = rows[row_idx - 1].clone();
                let new_y = {
                    let prev = previous_row.borrow();
                    prev.y() + prev.height() + self.node_node_spacing
                };
                current_row.borrow_mut().set_y(new_y);
            }
            let (_something_changed, compact_row_again) = Compaction::compact(
                row_idx,
                &mut rows,
                target_width,
                self.node_node_spacing,
                row_height_reevaluation,
            );
            if compact_row_again {
                let blocks = { current_row.borrow().children().clone() };
                for block in blocks {
                    let mut block_mut = block.borrow_mut();
                    block_mut.set_fixed(false);
                    block_mut.set_position_fixed(false);
                    block_mut.reset_block();
                }
                current_row.borrow_mut().reset_stacks();
                current_row.borrow_mut().set_width(target_width);
                if row_idx > 0 {
                    row_idx = row_idx.saturating_sub(1);
                }
            } else {
                self.adjust_width_and_height(&current_row);
                if row_idx + 1 < rows.len() {
                    let next_row = rows[row_idx + 1].clone();
                    let current_row_width = current_row.borrow().width();
                    let next_row_first_block_width =
                        next_row.borrow().get_first_block().borrow().width();
                    let increase =
                        current_row_width + self.node_node_spacing + next_row_first_block_width
                            - target_width;
                    self.potential_row_width_increase_max =
                        increase.max(self.potential_row_width_decrease_max);
                    self.potential_row_width_increase_min =
                        increase.min(self.potential_row_width_decrease_min);

                    let stacks = current_row.borrow().stacks().clone();
                    if !stacks.is_empty() {
                        let last_stack = stacks[stacks.len() - 1].clone();
                        let last_stack_width = last_stack.borrow().width();
                        let spacing = if stacks.len() <= 1 {
                            0.0
                        } else {
                            self.node_node_spacing
                        };
                        let decrease_value = last_stack_width + spacing;
                        self.potential_row_width_decrease_max =
                            self.potential_row_width_decrease_max.max(decrease_value);
                        self.potential_row_width_decrease_min =
                            self.potential_row_width_decrease_max.min(decrease_value);
                    }
                }

                if rows.len() == 1 {
                    let stacks = current_row.borrow().stacks().clone();
                    if let Some(last_stack) = stacks.last() {
                        let last_block = {
                            let stack = last_stack.borrow();
                            stack.blocks().last().cloned()
                        };
                        if let Some(last_block) = last_block {
                            let (last_block_width, rows_copy) = {
                                let block_guard = last_block.borrow();
                                (block_guard.width(), block_guard.rows().to_vec())
                            };
                            for block_row in rows_copy {
                                let decrease = last_block_width - block_row.width();
                                self.potential_row_width_decrease_max =
                                    self.potential_row_width_decrease_max.max(decrease);
                                self.potential_row_width_decrease_min =
                                    self.potential_row_width_decrease_min.min(decrease);
                                let increase = block_row.width() + self.node_node_spacing;
                                self.potential_row_width_increase_max =
                                    self.potential_row_width_increase_max.max(increase);
                                self.potential_row_width_increase_min =
                                    self.potential_row_width_increase_min.min(increase);
                            }
                        }
                    }
                }

                if progress_monitor.is_logging_enabled() {
                    progress_monitor.log_graph(layout_graph, &format!("Compacted row {}", row_idx));
                }
                row_idx += 1;
            }
        }

        let size = DrawingUtil::calculate_dimensions(&rows, self.node_node_spacing);

        let total_width = size.x.max(min_width - (padding.left + padding.right));
        let height = size.y.max(min_height - (padding.top + padding.bottom));
        let additional_height = height - size.y;

        set_property(
            layout_graph,
            InternalProperties::ADDITIONAL_HEIGHT,
            additional_height,
        );
        let rows_key = rows_storage::store_rows(layout_graph, rows.clone());
        set_property(layout_graph, InternalProperties::ROWS, rows_key);

        DrawingData::new(
            self.aspect_ratio,
            total_width,
            size.y + additional_height,
            DrawingDataDescriptor::WholeDrawing,
        )
    }

    fn adjust_width_and_height(&self, row: &RectRowRef) {
        let stacks = row.borrow().stacks().clone();
        let mut max_height: f64 = 0.0;
        let mut max_width: f64 = 0.0;
        for (index, stack) in stacks.iter().enumerate() {
            stack.borrow_mut().update_dimension();
            let stack_ref = stack.borrow();
            max_height = max_height.max(stack_ref.height());
            max_width += stack_ref.width()
                + if index > 0 {
                    self.node_node_spacing
                } else {
                    0.0
                };
        }
        let mut row_mut = row.borrow_mut();
        row_mut.set_height(max_height);
        row_mut.set_width(max_width);
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
