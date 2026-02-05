use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use super::RectRowRef;

pub struct DrawingUtil;

impl DrawingUtil {
    pub fn compute_scale_measure(width: f64, height: f64, desired_aspect_ratio: f64) -> f64 {
        (desired_aspect_ratio / width).min(1.0 / height)
    }

    pub fn reset_coordinates(nodes: &[ElkNodeRef]) {
        for node in nodes {
            let mut node_mut = node.borrow_mut();
            node_mut.connectable().shape().set_location(0.0, 0.0);
        }
    }

    pub fn calculate_dimensions(rows: &[RectRowRef], node_node_spacing: f64) -> KVector {
        let mut max_width: f64 = 0.0;
        let mut new_height: f64 = 0.0;
        let mut index = 0usize;
        for row in rows {
            let row_guard = row.borrow();
            max_width = max_width.max(row_guard.width());
            new_height += row_guard.height() + if index > 0 { node_node_spacing } else { 0.0 };
            index += 1;
        }
        KVector::with_values(max_width, new_height)
    }

    pub fn calculate_dimensions_from_nodes(nodes: &[ElkNodeRef]) -> KVector {
        let mut max_width: f64 = 0.0;
        let mut max_height: f64 = 0.0;
        for node in nodes {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            let width = shape.width() + shape.x();
            let height = shape.height() + shape.y();
            max_width = max_width.max(width);
            max_height = max_height.max(height);
        }
        KVector::with_values(max_width, max_height)
    }
}
