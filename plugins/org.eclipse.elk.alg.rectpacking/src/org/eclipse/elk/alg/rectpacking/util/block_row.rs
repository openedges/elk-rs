use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

#[derive(Clone)]
pub struct BlockRow {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    node_node_spacing: f64,
    rects: Vec<ElkNodeRef>,
}

impl BlockRow {
    pub fn new(x: f64, y: f64, node_node_spacing: f64) -> Self {
        BlockRow {
            x,
            y,
            width: 0.0,
            height: 0.0,
            node_node_spacing,
            rects: Vec::new(),
        }
    }

    pub fn add_rectangle(&mut self, rect: &ElkNodeRef) -> bool {
        {
            let mut rect_mut = rect.borrow_mut();
            let offset = if self.rects.is_empty() { 0.0 } else { self.node_node_spacing };
            rect_mut
                .connectable()
                .shape()
                .set_x(self.x + self.width + offset);
            rect_mut.connectable().shape().set_y(self.y);
            self.height = self.height.max(rect_mut.connectable().shape().height());
            self.width += rect_mut.connectable().shape().width() + offset;
        }
        self.rects.push(rect.clone());
        true
    }

    pub fn remove_rectangle(&mut self, rect: &ElkNodeRef, update: bool) {
        self.rects
            .retain(|item| !std::rc::Rc::ptr_eq(item, rect));
        if update {
            self.update_row();
        }
    }

    pub fn update_row(&mut self) {
        let mut width: f64 = 0.0;
        let mut height: f64 = 0.0;
        for rect in &self.rects {
            let mut rect_mut = rect.borrow_mut();
            rect_mut.connectable().shape().set_x(self.x + width);
            rect_mut.connectable().shape().set_y(self.y);
            width += rect_mut.connectable().shape().width() + self.node_node_spacing;
            height = height.max(rect_mut.connectable().shape().height() + self.node_node_spacing);
        }
        if !self.rects.is_empty() {
            self.width = width - self.node_node_spacing;
            self.height = height - self.node_node_spacing;
        } else {
            self.width = 0.0;
            self.height = 0.0;
        }
    }

    pub fn expand(&mut self, width_for_row: f64, additional_height_for_row: f64, index: usize) {
        let additional_width_for_rect = if self.rects.is_empty() {
            0.0
        } else {
            (width_for_row - self.width) / self.rects.len() as f64
        };
        let mut i = 0usize;
        self.height += additional_height_for_row;
        self.width = width_for_row;
        for rect in &self.rects {
            let (old_width, old_height) = {
                let mut rect_mut = rect.borrow_mut();
                let shape = rect_mut.connectable().shape();
                (shape.width(), shape.height())
            };
            {
                let mut rect_mut = rect.borrow_mut();
                let shape = rect_mut.connectable().shape();
                shape.set_x(shape.x() + i as f64 * additional_width_for_rect);
                shape.set_y(shape.y() + index as f64 * additional_height_for_row);
                shape.set_width(shape.width() + additional_width_for_rect);
                shape.set_height(self.height);
            }
            let new_width = {
                let mut rect_mut = rect.borrow_mut();
                rect_mut.connectable().shape().width()
            };
            let new_height = {
                let mut rect_mut = rect.borrow_mut();
                rect_mut.connectable().shape().height()
            };
            ElkUtil::translate((
                rect,
                &KVector::with_values(new_width, new_height),
                &KVector::with_values(old_width, old_height),
            ));
            i += 1;
        }
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn set_y(&mut self, y: f64) {
        self.y = y;
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn nodes(&self) -> &Vec<ElkNodeRef> {
        &self.rects
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn set_x(&mut self, x: f64) {
        self.x = x;
    }
}
