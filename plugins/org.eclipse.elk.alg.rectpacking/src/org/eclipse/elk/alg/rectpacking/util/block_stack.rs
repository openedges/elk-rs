use std::rc::Rc;

use super::{BlockRef, BlockStackRef};

#[derive(Clone)]
pub struct BlockStack {
    blocks: Vec<BlockRef>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    node_node_spacing: f64,
}

impl BlockStack {
    pub fn new(x: f64, y: f64, node_node_spacing: f64) -> Self {
        BlockStack {
            blocks: Vec::new(),
            x,
            y,
            width: 0.0,
            height: 0.0,
            node_node_spacing,
        }
    }

    pub fn add_block(stack_ref: &BlockStackRef, block: BlockRef) {
        {
            let mut block_mut = block.borrow_mut();
            block_mut.set_stack(Some(Rc::downgrade(stack_ref)));
        }
        let mut stack = stack_ref.borrow_mut();
        stack.width = stack.width.max(block.borrow().width());
        stack.height += block.borrow().height()
            + if stack.blocks.is_empty() {
                0.0
            } else {
                stack.node_node_spacing
            };
        stack.blocks.push(block);
    }

    pub fn update_dimension(&mut self) {
        let mut height: f64 = 0.0;
        let mut width: f64 = 0.0;
        for (index, block) in self.blocks.iter().enumerate() {
            width = width.max(block.borrow().width());
            height += block.borrow().height()
                + if index > 0 {
                    self.node_node_spacing
                } else {
                    0.0
                };
        }
        self.height = height;
        self.width = width;
    }

    pub fn set_location(&mut self, x: f64, y: f64) {
        let x_diff = x - self.x;
        let y_diff = y - self.y;
        for block in &self.blocks {
            let mut block_mut = block.borrow_mut();
            let new_x = block_mut.x() + x_diff;
            let new_y = block_mut.y() + y_diff;
            block_mut.set_location(new_x, new_y);
        }
        self.x = x;
        self.y = y;
    }

    pub fn get_width_for_fixed_height(&self, height: f64) -> f64 {
        if self.blocks.len() == 1 {
            return self.blocks[0].borrow().width_for_target_height(height);
        }

        let min_width = self.minimum_width();
        let mut upper_bound = self.width;
        let mut lower_bound = min_width;
        let mut viable_width = self.width;
        let mut new_width = (upper_bound - lower_bound) / 2.0 + lower_bound;
        while lower_bound + 1.0 < upper_bound {
            let mut total_height = 0.0;
            for block in &self.blocks {
                total_height += block.borrow().height_for_target_width(new_width);
            }
            if total_height < height {
                viable_width = new_width;
                upper_bound = new_width;
            } else {
                lower_bound = new_width;
            }
            new_width = (upper_bound - lower_bound) / 2.0 + lower_bound;
        }
        viable_width
    }

    pub fn place_rects_in(&mut self, target_width: f64) {
        let mut current_y = self.y;
        let mut current_height: f64 = 0.0;
        let mut current_width: f64 = 0.0;
        for block in &self.blocks {
            let mut block_mut = block.borrow_mut();
            block_mut.set_location(self.x, current_y);
            block_mut.place_rects_in(target_width);
            current_width = current_width.max(block_mut.width());
            current_y += block_mut.height() + self.node_node_spacing;
            current_height = current_y;
        }
        self.width = current_width;
        self.height = current_height;
    }

    pub fn expand(stack_ref: &BlockStackRef, additional_width: f64, additional_height: f64) {
        let blocks = stack_ref.borrow().blocks.clone();
        let stack_width = stack_ref.borrow().width;
        let additional_height_per_block = if blocks.is_empty() {
            0.0
        } else {
            additional_height / blocks.len() as f64
        };
        for (index, block) in blocks.iter().enumerate() {
            let mut block_mut = block.borrow_mut();
            let new_x = block_mut.x();
            let new_y = block_mut.y() + index as f64 * additional_height_per_block;
            block_mut.set_location(new_x, new_y);
            let expand_width = stack_width - block_mut.width() + additional_width;
            block_mut.expand(expand_width, additional_height_per_block);
        }
    }

    fn minimum_width(&self) -> f64 {
        let mut min_width: f64 = 0.0;
        for block in &self.blocks {
            min_width = min_width.max(block.borrow().min_width());
        }
        min_width
    }

    pub fn blocks(&self) -> &Vec<BlockRef> {
        &self.blocks
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
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
}
