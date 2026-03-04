use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkMath;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::optimization::IEvaluation;

#[derive(Default)]
pub struct EdgeLengthOptimization;

impl IEvaluation for EdgeLengthOptimization {
    fn evaluate(&self, root: &ElkNodeRef) -> f64 {
        // Pre-extract root geometry ONCE instead of per-edge
        let (root_cx, root_cy, root_w, root_h) = {
            let mut root_mut = root.borrow_mut();
            let shape = root_mut.connectable().shape();
            (
                shape.x() + shape.width() / 2.0,
                shape.y() + shape.height() / 2.0,
                shape.width(),
                shape.height(),
            )
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

            // Single borrow for target geometry
            let (target_cx, target_cy, target_w, target_h) = {
                let mut target_mut = target.borrow_mut();
                let shape = target_mut.connectable().shape();
                (
                    shape.x() + shape.width() / 2.0,
                    shape.y() + shape.height() / 2.0,
                    shape.width(),
                    shape.height(),
                )
            };

            let mut vector = KVector::new();
            vector.x = target_cx - root_cx;
            vector.y = target_cy - root_cy;
            let mut source_clip = KVector::with_values(vector.x, vector.y);
            ElkMath::clip_vector(&mut source_clip, root_w, root_h);
            vector.x -= source_clip.x;
            vector.y -= source_clip.y;

            let root_x = target_cx - vector.x;
            let root_y = target_cy - vector.y;

            let mut target_clip = KVector::with_values(vector.x, vector.y);
            ElkMath::clip_vector(&mut target_clip, target_w, target_h);
            vector.x -= target_clip.x;
            vector.y -= target_clip.y;

            let target_x = root_x + vector.x;
            let target_y = root_y + vector.y;

            let vector_x = target_x - root_x;
            let vector_y = target_y - root_y;
            edge_length += (vector_x * vector_x + vector_y * vector_y).sqrt();
        }
        edge_length
    }
}
