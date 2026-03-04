use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

#[derive(Default)]
pub struct AbstractRadiusExtensionCompaction {
    compaction_step: i32,
    spacing: f64,
}

impl AbstractRadiusExtensionCompaction {
    pub fn new() -> Self {
        AbstractRadiusExtensionCompaction {
            compaction_step: 1,
            spacing: 0.0,
        }
    }

    pub fn contract_layer(
        &self,
        root: &ElkNodeRef,
        layer_nodes: &[ElkNodeRef],
        is_contracting: bool,
    ) {
        // Cache root center — constant across all nodes
        let (parent_x, parent_y) = node_center(root);
        let step = self.compaction_step as f64;

        for node in layer_nodes {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            let half_w = shape.width() / 2.0;
            let half_h = shape.height() / 2.0;
            let x_pos = shape.x() + half_w;
            let y_pos = shape.y() + half_h;

            let dx = x_pos - parent_x;
            let dy = y_pos - parent_y;
            let length = (dx * dx + dy * dy).sqrt();

            let ux = dx * step / length;
            let uy = dy * step / length;

            if is_contracting {
                shape.set_x(x_pos - ux - half_w);
                shape.set_y(y_pos - uy - half_h);
            } else {
                shape.set_x(x_pos + ux - half_w);
                shape.set_y(y_pos + uy - half_h);
            }
        }
    }

    pub fn move_node(&self, root: &ElkNodeRef, node: &ElkNodeRef, distance: f64) {
        let (root_x, root_y) = node_center(root);
        let (node_x, node_y) = node_center(node);
        let difference_x = node_x - root_x;
        let difference_y = node_y - root_y;
        let length = (difference_x * difference_x + difference_y * difference_y).sqrt();
        let unit_x = difference_x / length;
        let unit_y = difference_y / length;

        let mut node_mut = node.borrow_mut();
        let shape = node_mut.connectable().shape();
        shape.set_x(shape.x() + unit_x * distance);
        shape.set_y(shape.y() + unit_y * distance);
    }

    pub fn overlap(&self, node1: &ElkNodeRef, node2: &ElkNodeRef) -> bool {
        let (x1, y1, width1, height1) = node_bounds(node1, self.spacing);
        let (x2, y2, width2, height2) = node_bounds(node2, self.spacing);
        let x_overlap = x1 < x2 + width2 && x2 < x1 + width1;
        let y_overlap = y1 < y2 + height2 && y2 < y1 + height1;
        x_overlap && y_overlap
    }

    pub fn overlap_layer(&self, nodes: &[ElkNodeRef]) -> bool {
        if nodes.len() < 2 {
            return false;
        }
        // Pre-extract bounds to avoid repeated borrows
        let bounds: Vec<(f64, f64, f64, f64)> =
            nodes.iter().map(|n| node_bounds(n, self.spacing)).collect();
        let mut overlapping = false;
        for i in 0..bounds.len() {
            let j = if i < bounds.len() - 1 { i + 1 } else { 0 };
            let (x1, y1, w1, h1) = bounds[i];
            let (x2, y2, w2, h2) = bounds[j];
            if x1 < x2 + w2 && x2 < x1 + w1 && y1 < y2 + h2 && y2 < y1 + h1 {
                overlapping = true;
            }
        }
        overlapping
    }

    pub fn get_compaction_step(&self) -> i32 {
        self.compaction_step
    }

    pub fn set_compaction_step(&mut self, compaction_step: i32) {
        self.compaction_step = compaction_step;
    }

    pub fn get_spacing(&self) -> f64 {
        self.spacing
    }

    pub fn set_spacing(&mut self, spacing: f64) {
        self.spacing = spacing;
    }
}

fn node_center(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (
        shape.x() + shape.width() / 2.0,
        shape.y() + shape.height() / 2.0,
    )
}

fn node_bounds(node: &ElkNodeRef, spacing: f64) -> (f64, f64, f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    let x = shape.x() - spacing / 2.0;
    let y = shape.y() - spacing / 2.0;
    let width = shape.width() + spacing;
    let height = shape.height() + spacing;
    (x, y, width, height)
}
