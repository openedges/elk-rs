use std::cell::RefCell;
use std::rc::Rc;

use super::{BlockRef, BlockStackRef, BlockStack};

#[derive(Clone)]
pub struct RectRow {
    height: f64,
    width: f64,
    y: f64,
    node_node_spacing: f64,
    children: Vec<BlockRef>,
    stacks: Vec<BlockStackRef>,
    potential_additional_width_to_get_last_block: f64,
}

impl RectRow {
    pub fn new(y: f64, node_node_spacing: f64) -> Self {
        RectRow {
            height: 0.0,
            width: 0.0,
            y,
            node_node_spacing,
            children: Vec::new(),
            stacks: Vec::new(),
            potential_additional_width_to_get_last_block: 0.0,
        }
    }

    pub fn notify_about_node_change(&mut self) {
        let mut total_stack_width = 0.0;
        let mut new_max_height = f64::NEG_INFINITY;
        for (index, child) in self.children.iter().enumerate() {
            let child_guard = child.borrow();
            total_stack_width += child_guard.width() + if index > 0 { self.node_node_spacing } else { 0.0 };
            new_max_height = new_max_height.max(child_guard.height());
        }
        if new_max_height == f64::NEG_INFINITY {
            new_max_height = 0.0;
        }
        self.width = total_stack_width;
        self.height = new_max_height;
    }

    pub fn expand(&mut self, width: f64, additional_height: f64) {
        if self.stacks.is_empty() {
            return;
        }
        let additional_width = width - self.width;
        let additional_width_per_stack = additional_width / self.stacks.len() as f64;
        for (index, stack) in self.stacks.iter().enumerate() {
            let stack_height = stack.borrow().height();
            let additional_height_for_stack = self.height - stack_height + additional_height;
            let mut stack_mut = stack.borrow_mut();
            let new_x = stack_mut.x() + index as f64 * additional_width_per_stack;
            let new_y = stack_mut.y();
            stack_mut.set_location(new_x, new_y);
            drop(stack_mut);
            BlockStack::expand(stack, additional_width_per_stack, additional_height_for_stack);
        }
        self.width = width;
        self.height += additional_height;
    }

    pub fn calculate_block_stacks(&mut self) {
        self.stacks.clear();
        let mut current_x = f64::NEG_INFINITY;
        for block in &self.children {
            let block_x = block.borrow().x();
            if (block_x - current_x).abs() > f64::EPSILON {
                let stack = Rc::new(RefCell::new(BlockStack::new(block_x, self.y, self.node_node_spacing)));
                BlockStack::add_block(&stack, block.clone());
                self.stacks.push(stack);
                current_x = block_x;
            } else if let Some(last_stack) = self.stacks.last() {
                BlockStack::add_block(last_stack, block.clone());
            }
        }
    }

    pub fn get_first_block(&self) -> BlockRef {
        self.children[0].clone()
    }

    pub fn get_last_block(&self) -> BlockRef {
        self.children[self.children.len() - 1].clone()
    }

    pub fn number_of_assigned_blocks(&self) -> usize {
        self.children.len()
    }

    pub fn add_block(&mut self, block: BlockRef) {
        let block_height = block.borrow().height();
        let block_width = block.borrow().width();
        self.height = self.height.max(block_height);
        self.width += block_width + if self.children.is_empty() { 0.0 } else { self.node_node_spacing };
        self.children.push(block);
    }

    pub fn remove_block(&mut self, block: &BlockRef) {
        self.children
            .retain(|child| !Rc::ptr_eq(child, block));
        let block_width = block.borrow().width();
        self.width -= block_width + if self.children.is_empty() { 0.0 } else { self.node_node_spacing };

        let mut new_max_height = f64::MIN;
        for child in &self.children {
            new_max_height = new_max_height.max(child.borrow().height());
        }
        if new_max_height == f64::MIN {
            new_max_height = 0.0;
        }
        self.height = new_max_height;
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn set_height(&mut self, height: f64) {
        self.height = height;
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn set_width(&mut self, width: f64) {
        self.width = width;
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn set_y(&mut self, y: f64) {
        let y_change = y - self.y;
        for stack in &self.stacks {
            let mut stack_mut = stack.borrow_mut();
            let new_x = stack_mut.x();
            let new_y = stack_mut.y() + y_change;
            stack_mut.set_location(new_x, new_y);
        }
        self.y = y;
    }

    pub fn children(&self) -> &Vec<BlockRef> {
        &self.children
    }

    pub fn stacks(&self) -> &Vec<BlockStackRef> {
        &self.stacks
    }

    pub fn add_stack(&mut self, stack: BlockStackRef) {
        self.stacks.push(stack);
    }

    pub fn potential_additional_width_to_get_last_block(&self) -> f64 {
        self.potential_additional_width_to_get_last_block
    }

    pub fn set_potential_additional_width_to_get_last_block(&mut self, value: f64) {
        self.potential_additional_width_to_get_last_block = value;
    }

    pub fn reset_stacks(&mut self) {
        self.stacks.clear();
    }
}
