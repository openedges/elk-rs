use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::optimization::IEvaluation;

#[derive(Default)]
pub struct EdgeLengthPositionOptimization;

impl IEvaluation for EdgeLengthPositionOptimization {
    fn evaluate(&self, root: &ElkNodeRef) -> f64 {
        // Pre-extract root geometry ONCE instead of per-edge
        let (root_base_x, root_base_y, root_w, root_h) = {
            let mut root_mut = root.borrow_mut();
            let shape = root_mut.connectable().shape();
            (shape.x(), shape.y(), shape.width(), shape.height())
        };

        let mut edge_length = 0.0;
        for edge in ElkGraphUtil::all_outgoing_edges(root) {
            let target_shape = {
                let edge_borrow = edge.borrow();
                edge_borrow.targets_ro().get(0)
            };
            let Some(target_shape) = target_shape else {
                continue;
            };
            let Some(target) = ElkGraphUtil::connectable_shape_to_node(&target_shape) else {
                continue;
            };

            // Single borrow for target center + position
            let (target_x, target_y, position) = {
                let mut target_mut = target.borrow_mut();
                let shape = target_mut.connectable().shape();
                let cx = shape.x() + shape.width() / 2.0;
                let cy = shape.y() + shape.height() / 2.0;
                let pos = target_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(CoreOptions::POSITION)
                    .unwrap_or_else(KVector::new);
                (cx, cy, pos)
            };

            let root_x = root_base_x + position.x + root_w / 2.0;
            let root_y = root_base_y + position.y + root_h;

            let vector_x = target_x - root_x;
            let vector_y = target_y - root_y;
            edge_length += (vector_x * vector_x + vector_y * vector_y).sqrt();
        }
        edge_length
    }
}
