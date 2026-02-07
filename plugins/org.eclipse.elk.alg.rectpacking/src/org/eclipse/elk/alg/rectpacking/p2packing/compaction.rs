use std::rc::Rc;

use crate::org::eclipse::elk::alg::rectpacking::options::RectPackingOptions;
use crate::org::eclipse::elk::alg::rectpacking::p2packing::InitialPlacement;
use crate::org::eclipse::elk::alg::rectpacking::util::{BlockRef, BlockStack, BlockStackRef, RectRowRef};

pub struct Compaction;

impl Compaction {
    pub fn compact(
        row_idx: usize,
        rows: &mut Vec<RectRowRef>,
        bounding_width: f64,
        node_node_spacing: f64,
        row_height_reevaluation: bool,
    ) -> (bool, bool) {
        let mut something_was_changed = false;
        let mut compact_row_again = false;
        let next_row_index = row_idx + 1;
        let row = rows[row_idx].clone();
        let mut current_stack: Option<BlockStackRef> = None;

        let mut block_id = 0usize;
        while block_id < row.borrow().number_of_assigned_blocks() {
            let block = { row.borrow().children()[block_id].clone() };
            if block.borrow().is_fixed() {
                block_id += 1;
                continue;
            }
            if block.borrow().children().is_empty() {
                eprintln!("There should not be an empty block. Empty blocks are directly removed.");
                row.borrow_mut().remove_block(&block);
                something_was_changed = true;
                continue;
            }

            if !block.borrow().is_position_fixed() {
                if let Some(stack) = current_stack.as_ref() {
                    stack.borrow_mut().update_dimension();
                }
                let stack_x = current_stack
                    .as_ref()
                    .map(|stack| stack.borrow().x() + stack.borrow().width() + node_node_spacing)
                    .unwrap_or(0.0);
                let row_y = row.borrow().y();
                let new_stack = Rc::new(std::cell::RefCell::new(BlockStack::new(
                    stack_x,
                    row_y,
                    node_node_spacing,
                )));
                block
                    .borrow_mut()
                    .set_location(new_stack.borrow().x() + new_stack.borrow().width(), row_y);
                row.borrow_mut().add_stack(new_stack.clone());
                BlockStack::add_block(&new_stack, block.clone());
                block.borrow_mut().set_position_fixed(true);
                current_stack = Some(new_stack);
            }

            let mut next_block = Self::get_next_block(rows, &row, block_id, next_row_index);
            let was_from_next_row = next_block
                .as_ref()
                .and_then(|next| next.borrow().parent_row())
                .is_some_and(|parent| !Rc::ptr_eq(&parent, &row));

            if let Some(next_block_ref) = next_block.clone() {
                if !next_block_ref.borrow().children().is_empty()
                    && !node_in_new_row(&next_block_ref.borrow().children()[0])
                {
                    Self::use_row_width(&block, bounding_width);
                    something_was_changed |= Self::absorb_blocks(
                        &row,
                        &block,
                        &next_block_ref,
                        bounding_width,
                        node_node_spacing,
                    );
                } else {
                    row.borrow_mut().remove_block(&next_block_ref);
                    break;
                }

                if next_block_ref.borrow().children().is_empty() {
                    if rows.len() > next_row_index {
                        rows[next_row_index]
                            .borrow_mut()
                            .remove_block(&next_block_ref);
                    }
                    next_block = None;
                    while rows.len() > next_row_index
                        && rows[next_row_index].borrow().children().is_empty()
                    {
                        rows.remove(next_row_index);
                    }
                }
                if next_block.is_none() {
                    continue;
                }

                if let Some(next_block_ref) = next_block.clone() {
                    if !node_in_new_row(&next_block_ref.borrow().children()[0])
                        && Self::place_below(
                            rows,
                            &row,
                            &block,
                            &next_block_ref,
                            was_from_next_row,
                            bounding_width,
                            next_row_index,
                            node_node_spacing,
                        )
                    {
                        something_was_changed = true;
                        block_id += 1;
                        continue;
                    }

                    if was_from_next_row {
                        let old_row_height = row.borrow().height();
                        let next_block_min_height = next_block_ref.borrow().min_height();
                        if !node_in_new_row(&next_block_ref.borrow().children()[0])
                            && Self::place_beside(
                                rows,
                                &row,
                                &block,
                                &next_block_ref,
                                was_from_next_row,
                                bounding_width,
                                next_row_index,
                                node_node_spacing,
                                row_height_reevaluation,
                            )
                        {
                            something_was_changed = true;
                            if old_row_height < next_block_min_height {
                                compact_row_again = true;
                                next_block_ref.borrow_mut().set_parent_row(row.clone());
                                break;
                            }
                            block_id += 1;
                            continue;
                        } else if Self::use_row_height(&row, &block) {
                            block.borrow_mut().set_fixed(true);
                            something_was_changed = true;
                            block_id += 1;
                            continue;
                        }
                    } else if Self::use_row_height(&row, &block) {
                        block.borrow_mut().set_fixed(true);
                        something_was_changed = true;
                        block_id += 1;
                        continue;
                    }

                    if something_was_changed {
                        block_id += 1;
                        continue;
                    }
                }
            }

            if Self::use_row_height(&row, &block) {
                block.borrow_mut().set_fixed(true);
                something_was_changed = true;
                if let Some(next_block_ref) = next_block {
                    next_block_ref.borrow_mut().set_position_fixed(false);
                }
                block_id += 1;
                continue;
            } else if let Some(stack) = block.borrow().stack() {
                stack.borrow_mut().update_dimension();
            }

            block_id += 1;
        }

        (something_was_changed, compact_row_again)
    }

    fn get_next_block(
        rows: &[RectRowRef],
        row: &RectRowRef,
        block_id: usize,
        next_row_index: usize,
    ) -> Option<BlockRef> {
        let row_guard = row.borrow();
        if block_id < row_guard.number_of_assigned_blocks().saturating_sub(1) {
            Some(row_guard.children()[block_id + 1].clone())
        } else if next_row_index < rows.len()
            && !rows[next_row_index].borrow().children().is_empty()
        {
            Some(rows[next_row_index].borrow().children()[0].clone())
        } else {
            None
        }
    }

    fn use_row_height(row: &RectRowRef, block: &BlockRef) -> bool {
        let row_height = row.borrow().height();
        let block_height = block.borrow().height();
        let stack = block.borrow().stack();
        let Some(stack) = stack else { return false; };
        let previous_width = stack.borrow().width();
        if block_height < row_height {
            let target_width = stack.borrow().get_width_for_fixed_height(row_height);
            if stack.borrow().width() > target_width {
                stack.borrow_mut().place_rects_in(target_width);
                return previous_width != stack.borrow().width();
            }
        }
        false
    }

    fn use_row_width(block: &BlockRef, bounding_width: f64) {
        let block_x = block.borrow().x();
        block.borrow_mut().place_rects_in(bounding_width - block_x);
        if let Some(stack) = block.borrow().stack() {
            stack.borrow_mut().update_dimension();
        }
    }

    fn absorb_blocks(
        row: &RectRowRef,
        block: &BlockRef,
        next_block: &BlockRef,
        bounding_width: f64,
        node_node_spacing: f64,
    ) -> bool {
        let mut something_was_changed = false;
        let mut rect = { next_block.borrow().children()[0].clone() };
        while InitialPlacement::place_rect_in_block(
            row,
            block,
            &rect,
            bounding_width,
            node_node_spacing,
        ) {
            something_was_changed = true;
            next_block.borrow_mut().remove_child(&rect);
            if next_block.borrow().children().is_empty() {
                break;
            }
            rect = next_block.borrow().children()[0].clone();
        }

        if next_block.borrow().children().is_empty() {
            if let Some(parent_row) = next_block.borrow().parent_row() {
                parent_row.borrow_mut().remove_block(next_block);
            }
        }
        if something_was_changed {
            if let Some(stack) = block.borrow().stack() {
                stack.borrow_mut().update_dimension();
            }
        }
        something_was_changed
    }

    #[allow(clippy::too_many_arguments)]
    fn place_below(
        rows: &mut Vec<RectRowRef>,
        row: &RectRowRef,
        block: &BlockRef,
        next_block: &BlockRef,
        was_from_next_row: bool,
        bounding_width: f64,
        next_row_index: usize,
        node_node_spacing: f64,
    ) -> bool {
        let remaining_width = bounding_width - block.borrow().x();
        if next_block.borrow().min_width() + node_node_spacing > remaining_width {
            return false;
        }
        let current_block_min_height =
            block.borrow().y() - row.borrow().y() + block.borrow().height_for_target_width(remaining_width);
        let next_block_min_height = next_block.borrow().height_for_target_width(remaining_width);
        if current_block_min_height + node_node_spacing + next_block_min_height <= row.borrow().height() {
            let block_x = block.borrow().x();
            block.borrow_mut().place_rects_in(bounding_width - block_x);
            block.borrow_mut().set_fixed(true);
            next_block
                .borrow_mut()
                .place_rects_in(bounding_width - block_x);
            next_block.borrow_mut().set_location(
                block_x,
                block.borrow().y() + block.borrow().height() + node_node_spacing,
            );
            next_block.borrow_mut().set_position_fixed(true);
            if let Some(stack) = block.borrow().stack() {
                BlockStack::add_block(&stack, next_block.clone());
            }

            if was_from_next_row {
                row.borrow_mut().add_block(next_block.clone());
                next_block.borrow_mut().set_parent_row(row.clone());
                if rows.len() > next_row_index {
                    rows[next_row_index]
                        .borrow_mut()
                        .remove_block(next_block);
                    if rows[next_row_index].borrow().children().is_empty() {
                        rows.remove(next_row_index);
                    }
                }
            }
            return true;
        }
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn place_beside(
        rows: &mut Vec<RectRowRef>,
        row: &RectRowRef,
        block: &BlockRef,
        next_block: &BlockRef,
        _was_from_next_row: bool,
        bounding_width: f64,
        next_row_index: usize,
        node_node_spacing: f64,
        row_height_reevaluation: bool,
    ) -> bool {
        let stack = match block.borrow().stack() {
            Some(stack) => stack,
            None => return false,
        };
        let row_height = row.borrow().height();
        let current_block_min_width = stack
            .borrow()
            .get_width_for_fixed_height(row.borrow().y() + row_height - stack.borrow().y());
        let next_block_min_height = next_block.borrow().min_height();
        let should_row_height_be_reevaluated = next_block_min_height > row_height && row_height_reevaluation;
        let mut target_width_of_next_block = bounding_width -
            (stack.borrow().x() + current_block_min_width - node_node_spacing);
        let next_block_height = next_block.borrow().height_for_target_width(target_width_of_next_block);
        if should_row_height_be_reevaluated && next_block_height > next_block_min_height {
            return false;
        }

        if should_row_height_be_reevaluated {
            let stacks = row.borrow().stacks().clone();
            let mut potential_width = 0.0;
            for stack in stacks {
                potential_width += stack
                    .borrow()
                    .get_width_for_fixed_height(next_block_min_height)
                    + node_node_spacing;
            }
            target_width_of_next_block = bounding_width - potential_width;
        }

        if target_width_of_next_block < next_block.borrow().min_width() {
            return false;
        }

        let last_row_optimization = next_row_index == rows.len() - 1
            && target_width_of_next_block >= rows[next_row_index].borrow().width();

        if !should_row_height_be_reevaluated
            && next_block_height > row_height
            && !last_row_optimization
        {
            return false;
        }

        if last_row_optimization || should_row_height_be_reevaluated || next_block_height <= row_height {
            if last_row_optimization && next_block_height > row_height {
                block.borrow_mut().set_height(next_block_height);
                let target_width = block.borrow().width_for_target_height(next_block_height);
                block.borrow_mut().place_rects_in(target_width);
            } else {
                stack.borrow_mut().place_rects_in(current_block_min_width);
                block.borrow_mut().set_fixed(true);
            }

            next_block
                .borrow_mut()
                .place_rects_in(bounding_width - (block.borrow().x() + block.borrow().width()));
            next_block.borrow_mut().set_location(
                stack.borrow().x() + stack.borrow().width(),
                row.borrow().y(),
            );
            row.borrow_mut().add_block(next_block.clone());

            if rows.len() > next_row_index {
                rows[next_row_index]
                    .borrow_mut()
                    .remove_block(next_block);
                if rows[next_row_index].borrow().children().is_empty() {
                    rows.remove(next_row_index);
                }
            }
            return true;
        }
        false
    }
}

fn node_in_new_row(node: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef) -> bool {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(RectPackingOptions::IN_NEW_ROW)
        .unwrap_or(false)
}
