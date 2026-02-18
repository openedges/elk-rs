use std::cell::RefCell;
use std::rc::Rc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::options::RectPackingOptions;
use crate::org::eclipse::elk::alg::rectpacking::util::{Block, BlockRef, RectRow, RectRowRef};

pub struct InitialPlacement;

impl InitialPlacement {
    pub fn place(
        rectangles: &[ElkNodeRef],
        bounding_width: f64,
        node_node_spacing: f64,
    ) -> Vec<RectRowRef> {
        let mut rows: Vec<RectRowRef> = Vec::new();
        let mut row = Rc::new(RefCell::new(RectRow::new(0.0, node_node_spacing)));
        let mut drawing_height = 0.0;

        let block = Rc::new(RefCell::new(Block::new(0.0, 0.0, &row, node_node_spacing)));
        row.borrow_mut().add_block(block.clone());
        let mut current_width = 0.0;
        let mut current_block = block;

        for rect in rectangles {
            let rect_width = {
                let mut rect_mut = rect.borrow_mut();
                rect_mut.connectable().shape().width()
            };

            let in_new_row = {
                let mut rect_mut = rect.borrow_mut();
                rect_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(RectPackingOptions::IN_NEW_ROW)
            }
            .unwrap_or(false);

            let is_block_empty = current_block.borrow().children().is_empty();
            let potential_row_width = current_width
                + rect_width
                + if is_block_empty {
                    0.0
                } else {
                    node_node_spacing
                };

            if potential_row_width > bounding_width || in_new_row {
                drawing_height += row.borrow().height() + node_node_spacing;
                rows.push(row.clone());
                row = Rc::new(RefCell::new(RectRow::new(
                    drawing_height,
                    node_node_spacing,
                )));
                current_block = Rc::new(RefCell::new(Block::new(
                    0.0,
                    row.borrow().y(),
                    &row,
                    node_node_spacing,
                )));
                row.borrow_mut().add_block(current_block.clone());
            }

            let row_height_reevaluation = rect
                .borrow()
                .parent()
                .and_then(|parent| {
                    let mut parent_mut = parent.borrow_mut();
                    parent_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .get_property(
                            RectPackingOptions::PACKING_COMPACTION_ROW_HEIGHT_REEVALUATION,
                        )
                })
                .unwrap_or(false);

            if current_block.borrow().children().is_empty()
                || (!row_height_reevaluation
                    && Self::is_similar_height(&current_block, rect, node_node_spacing))
            {
                current_block.borrow_mut().add_child(rect.clone());
            } else {
                let new_block_x = {
                    let block = current_block.borrow();
                    block.x() + block.width() + node_node_spacing
                };
                let new_block = Rc::new(RefCell::new(Block::new(
                    new_block_x,
                    row.borrow().y(),
                    &row,
                    node_node_spacing,
                )));
                row.borrow_mut().add_block(new_block.clone());
                new_block.borrow_mut().add_child(rect.clone());
                current_block = new_block;
            }

            current_width = {
                let mut rect_mut = rect.borrow_mut();
                let shape = rect_mut.connectable().shape();
                shape.x() + shape.width()
            };
        }
        rows.push(row);
        rows
    }

    pub fn place_rect_in_block(
        row: &RectRowRef,
        block: &BlockRef,
        rect: &ElkNodeRef,
        bounding_width: f64,
        node_node_spacing: f64,
    ) -> bool {
        if Self::is_similar_height(block, rect, node_node_spacing) {
            let (last_row_new_x, last_row_y) = {
                let block_guard = block.borrow();
                (block_guard.last_row_new_x(), block_guard.last_row_y())
            };
            let rect_width = {
                let mut rect_mut = rect.borrow_mut();
                rect_mut.connectable().shape().width()
            };
            let rect_height = {
                let mut rect_mut = rect.borrow_mut();
                rect_mut.connectable().shape().height()
            };
            let row_height = row.borrow().height();

            if last_row_new_x + rect_width + node_node_spacing <= bounding_width
                && (last_row_y - row.borrow().y() + rect_height <= row_height
                    || row.borrow().children().len() == 1)
            {
                block.borrow_mut().add_child(rect.clone());
                return true;
            } else if block.borrow().x() + rect_width <= bounding_width
                && block.borrow().y() + block.borrow().height() + rect_height + node_node_spacing
                    <= row.borrow().y() + row_height
            {
                block.borrow_mut().add_child_in_new_row(rect.clone());
                return true;
            }
        }
        false
    }

    pub fn is_similar_height(block: &BlockRef, rect: &ElkNodeRef, _node_node_spacing: f64) -> bool {
        let rect_height = {
            let mut rect_mut = rect.borrow_mut();
            rect_mut.connectable().shape().height()
        };
        let block_guard = block.borrow();
        if rect_height >= block_guard.smallest_rect_height()
            && rect_height <= block_guard.min_height()
        {
            true
        } else {
            block_guard.average_height() * 0.5 <= rect_height
                && block_guard.average_height() * 1.5 >= rect_height
        }
    }
}
