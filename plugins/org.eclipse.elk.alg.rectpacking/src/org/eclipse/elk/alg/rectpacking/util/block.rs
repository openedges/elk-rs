use std::cell::RefCell;
use std::rc::{Rc, Weak};

use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkRectangle;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use super::{BlockRow, BlockStackRef, RectRowRef};

#[derive(Clone)]
pub struct Block {
    smallest_rect_width: f64,
    min_width: f64,
    width: f64,
    min_height: f64,
    smallest_rect_height: f64,
    average_height: f64,
    max_height: f64,
    height: f64,
    children: Vec<ElkNodeRef>,
    rows: Vec<BlockRow>,
    x: f64,
    y: f64,
    parent_row: Option<Weak<RefCell<super::RectRow>>>,
    stack: Option<Weak<RefCell<super::BlockStack>>>,
    node_node_spacing: f64,
    fixed: bool,
    position_fixed: bool,
}

impl Block {
    pub fn new(x: f64, y: f64, parent_row: &RectRowRef, node_node_spacing: f64) -> Self {
        Block {
            smallest_rect_width: f64::INFINITY,
            min_width: 0.0,
            width: 0.0,
            min_height: 0.0,
            smallest_rect_height: f64::INFINITY,
            average_height: 0.0,
            max_height: 0.0,
            height: 0.0,
            children: Vec::new(),
            rows: Vec::new(),
            x,
            y,
            parent_row: Some(Rc::downgrade(parent_row)),
            stack: None,
            node_node_spacing,
            fixed: false,
            position_fixed: false,
        }
    }

    pub fn add_child(&mut self, rect: ElkNodeRef) {
        if self.rows.is_empty() {
            self.rows.push(BlockRow::new(self.x, self.y, self.node_node_spacing));
        }
        self.children.push(rect.clone());
        if let Some(last_row) = self.rows.last_mut() {
            last_row.add_rectangle(&rect);
        }
        self.adjust_size_add(&rect);
    }

    pub fn add_child_in_new_row(&mut self, rect: ElkNodeRef) {
        self.children.push(rect.clone());
        let last_row_y = if let Some(last_row) = self.rows.last() {
            last_row.y() + last_row.height() + self.node_node_spacing
        } else {
            self.y
        };
        self.rows.push(BlockRow::new(self.x, last_row_y, self.node_node_spacing));
        if let Some(last_row) = self.rows.last_mut() {
            last_row.add_rectangle(&rect);
        }
        self.adjust_size_add(&rect);
    }

    pub fn remove_child(&mut self, rect: &ElkNodeRef) {
        self.children
            .retain(|child| !Rc::ptr_eq(child, rect));
        let mut row_index = None;
        for (index, row) in self.rows.iter_mut().enumerate() {
            if row.nodes().iter().any(|node| Rc::ptr_eq(node, rect)) {
                row.remove_rectangle(rect, true);
                if row.nodes().is_empty() {
                    row_index = Some(index);
                }
                break;
            }
        }
        if let Some(index) = row_index {
            self.rows.remove(index);
        }
        self.adjust_size_after_remove();
    }

    pub fn set_location(&mut self, x: f64, y: f64) {
        let x_change = x - self.x;
        let y_change = y - self.y;
        for rect in &self.children {
            let mut rect_mut = rect.borrow_mut();
            let shape = rect_mut.connectable().shape();
            shape.set_location(shape.x() + x_change, shape.y() + y_change);
        }
        for row in &mut self.rows {
            row.set_x(row.x() + x_change);
            row.set_y(row.y() + y_change);
        }
        self.x = x;
        self.y = y;
    }

    fn adjust_size_add(&mut self, rect: &ElkNodeRef) {
        let rect_width = {
            let mut rect_mut = rect.borrow_mut();
            rect_mut.connectable().shape().width()
        };
        let rect_height = {
            let mut rect_mut = rect.borrow_mut();
            rect_mut.connectable().shape().height()
        };
        let last_row_width = self
            .rows
            .last()
            .map(|row| row.width())
            .unwrap_or(0.0);

        self.smallest_rect_width = self.smallest_rect_width.min(rect_width);
        self.width = self.width.max(last_row_width);
        self.min_width = self
            .min_width
            .max(rect_width + if self.children.len() == 1 { 0.0 } else { self.node_node_spacing });

        self.smallest_rect_height = self.smallest_rect_height.min(rect_height);
        self.max_height += rect_height + if self.children.len() == 1 { 0.0 } else { self.node_node_spacing };
        self.min_height = self.min_height.max(rect_height);
        let mut total_height = if self.rows.len() > 0 { (self.rows.len() - 1) as f64 * self.node_node_spacing } else { 0.0 };
        for row in &self.rows {
            total_height += row.height();
        }
        self.height = total_height;
        let child_count = self.children.len() as f64;
        if child_count > 0.0 {
            self.average_height =
                self.max_height / child_count - self.node_node_spacing * ((child_count - 1.0) / child_count);
        }
        self.notify_parent();
    }

    pub fn width_for_target_height(&self, height: f64) -> f64 {
        if self.max_height <= height {
            return self.min_width;
        }
        if self.place_rects_in_width_height(self.min_width, height, false) {
            return self.min_width;
        }

        let mut upper_bound = self.width;
        let mut lower_bound = self.min_width;
        let mut viable_width = self.width;
        let mut new_width = (upper_bound - lower_bound) / 2.0 + lower_bound;
        while lower_bound + 1.0 < upper_bound {
            if self.place_rects_in_width_height(new_width, height, false) {
                viable_width = new_width;
                upper_bound = new_width;
            } else {
                lower_bound = new_width;
            }
            new_width = (upper_bound - lower_bound) / 2.0 + lower_bound;
        }
        viable_width
    }

    pub fn height_for_target_width(&self, width: f64) -> f64 {
        let mut temp = self.clone();
        let bounds = temp.place_rects_in_internal(width, false);
        bounds.height
    }

    fn place_rects_in_internal(&mut self, width: f64, place_rects: bool) -> ElkRectangle {
        let mut current_x = 0.0;
        let mut current_y = self.y;
        let mut current_width: f64 = 0.0;
        let mut current_height: f64 = 0.0;
        let mut max_height_in_row: f64 = 0.0;
        let mut width_in_row: f64 = 0.0;
        let mut row = 0usize;
        let mut rows = if place_rects {
            vec![BlockRow::new(self.x, self.y, self.node_node_spacing)]
        } else {
            Vec::new()
        };
        let mut index = 0usize;
        for rect in &self.children {
            let rect_width = rect.borrow_mut().connectable().shape().width();
            let rect_height = rect.borrow_mut().connectable().shape().height();
            if current_x + rect_width + if index > 0 { self.node_node_spacing } else { 0.0 } > width
                && max_height_in_row > 0.0
            {
                current_x = 0.0;
                current_y += max_height_in_row + self.node_node_spacing;
                current_width = current_width.max(width_in_row);
                current_height += max_height_in_row + self.node_node_spacing;
                max_height_in_row = 0.0;
                width_in_row = 0.0;
                if place_rects {
                    row += 1;
                    rows.push(BlockRow::new(self.x, current_y, self.node_node_spacing));
                }
                index = 0;
            }
            width_in_row += rect_width + if index > 0 { self.node_node_spacing } else { 0.0 };
            max_height_in_row = max_height_in_row.max(rect_height);
            if place_rects {
                if let Some(current_row) = rows.get_mut(row) {
                    current_row.add_rectangle(rect);
                }
            }
            current_x += rect_width + if index > 0 { self.node_node_spacing } else { 0.0 };
            index += 1;
        }
        current_width = current_width.max(width_in_row);
        current_height += max_height_in_row;
        if place_rects {
            self.rows = rows;
            self.width = current_width;
            self.height = current_height;
            self.notify_parent();
        }
        ElkRectangle::with_values(self.x, self.y, current_width, current_height)
    }

    fn place_rects_in_width_height(&self, width: f64, height: f64, place_rects: bool) -> bool {
        if place_rects {
            return false;
        }
        let mut temp = self.clone();
        let bounds = temp.place_rects_in_internal(width, false);
        bounds.width <= width && bounds.height <= height
    }

    pub fn place_rects_in_width_height_public(&mut self, width: f64, height: f64) -> bool {
        let bounds = self.place_rects_in_internal(width, true);
        bounds.width <= width && bounds.height <= height
    }

    pub fn place_rects_in(&mut self, width: f64) -> bool {
        let old_width = self.width;
        let old_height = self.height;
        let bounds = self.place_rects_in_internal(width, true);
        bounds.width != old_width || bounds.height != old_height
    }

    fn adjust_size_after_remove(&mut self) {
        let mut new_width: f64 = 0.0;
        let mut new_height: f64 = 0.0;
        let mut rows_to_delete = Vec::new();
        for (index, row) in self.rows.iter().enumerate() {
            if row.nodes().is_empty() {
                rows_to_delete.push(index);
            } else {
                new_width = new_width.max(row.width());
                new_height += row.height() + if index > 0 { self.node_node_spacing } else { 0.0 };
            }
        }
        for index in rows_to_delete.into_iter().rev() {
            self.rows.remove(index);
        }
        self.height = new_height;
        self.width = new_width;

        self.min_width = 0.0;
        self.min_height = 0.0;
        self.max_height = 0.0;
        self.smallest_rect_height = f64::INFINITY;
        self.smallest_rect_width = f64::INFINITY;
        for rect in &self.children {
            let rect_width = rect.borrow_mut().connectable().shape().width();
            let rect_height = rect.borrow_mut().connectable().shape().height();
            self.smallest_rect_width = self.smallest_rect_width.min(rect_width);
            self.min_width = self.min_width.max(rect_width);
            self.min_height = self.min_height.max(rect_height);
            self.smallest_rect_height = self.smallest_rect_height.min(rect_height);
            self.max_height += rect_height + self.node_node_spacing;
        }
        let child_count = self.children.len() as f64;
        if child_count > 0.0 {
            self.average_height =
                self.max_height / child_count - self.node_node_spacing * ((child_count - 1.0) / child_count);
        }
        self.notify_parent();
    }

    pub fn expand(&mut self, additional_width_per_block: f64, additional_height_for_block: f64) {
        let width_for_row = self.width + additional_width_per_block;
        self.width += additional_width_per_block;
        self.height += additional_height_for_block;
        let additional_height_for_row = additional_height_for_block / self.rows.len() as f64;
        for (index, row) in self.rows.iter_mut().enumerate() {
            row.expand(width_for_row, additional_height_for_row, index);
        }
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn set_width(&mut self, width: f64) {
        self.width = width;
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn set_height(&mut self, height: f64) {
        self.height = height;
    }

    pub fn children(&self) -> &Vec<ElkNodeRef> {
        &self.children
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn parent_row(&self) -> Option<RectRowRef> {
        self.parent_row.as_ref().and_then(|row| row.upgrade())
    }

    pub fn set_parent_row(&mut self, parent_row: RectRowRef) {
        self.parent_row = Some(Rc::downgrade(&parent_row));
    }

    pub fn min_width(&self) -> f64 {
        self.min_width
    }

    pub fn min_height(&self) -> f64 {
        self.min_height
    }

    pub fn max_height(&self) -> f64 {
        self.max_height
    }

    pub fn smallest_rect_height(&self) -> f64 {
        self.smallest_rect_height
    }

    pub fn average_height(&self) -> f64 {
        self.average_height
    }

    pub fn is_fixed(&self) -> bool {
        self.fixed
    }

    pub fn set_fixed(&mut self, fixed: bool) {
        self.fixed = fixed;
    }

    pub fn is_position_fixed(&self) -> bool {
        self.position_fixed
    }

    pub fn set_position_fixed(&mut self, position_fixed: bool) {
        self.position_fixed = position_fixed;
    }

    pub fn rows(&self) -> &Vec<BlockRow> {
        &self.rows
    }

    pub fn last_row_new_x(&self) -> f64 {
        let last_row = self.last_row();
        last_row.x() + last_row.width()
    }

    pub fn last_row_y(&self) -> f64 {
        let last_row = self.last_row();
        last_row.y()
    }

    pub fn last_row(&self) -> &BlockRow {
        &self.rows[self.rows.len() - 1]
    }

    pub fn stack(&self) -> Option<BlockStackRef> {
        self.stack.as_ref().and_then(|stack| stack.upgrade())
    }

    pub fn set_stack(&mut self, stack: Option<Weak<RefCell<super::BlockStack>>>) {
        self.stack = stack;
    }

    pub fn smallest_rect_width(&self) -> f64 {
        self.smallest_rect_width
    }

    pub fn set_smallest_rect_width(&mut self, value: f64) {
        self.smallest_rect_width = value;
    }

    pub fn reset_block(&mut self) {
        self.adjust_size_after_remove();
    }

    fn notify_parent(&self) {
        if let Some(parent) = self.parent_row.as_ref().and_then(|row| row.upgrade()) {
            parent.borrow_mut().notify_about_node_change();
        }
    }
}
