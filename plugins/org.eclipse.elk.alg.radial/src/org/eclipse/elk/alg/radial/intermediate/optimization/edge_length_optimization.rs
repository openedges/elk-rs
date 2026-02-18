use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkMath;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::optimization::IEvaluation;

#[derive(Default)]
pub struct EdgeLengthOptimization;

impl IEvaluation for EdgeLengthOptimization {
    fn evaluate(&self, root: &ElkNodeRef) -> f64 {
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

            let (mut target_x, mut target_y) = node_center(&target);
            let (mut root_x, mut root_y) = node_center(root);

            let mut vector = KVector::new();
            vector.x = target_x - root_x;
            vector.y = target_y - root_y;
            let (root_width, root_height) = node_size(root);
            let mut source_clip = KVector::with_values(vector.x, vector.y);
            ElkMath::clip_vector(&mut source_clip, root_width, root_height);
            vector.x -= source_clip.x;
            vector.y -= source_clip.y;

            root_x = target_x - vector.x;
            root_y = target_y - vector.y;

            let (target_width, target_height) = node_size(&target);
            let mut target_clip = KVector::with_values(vector.x, vector.y);
            ElkMath::clip_vector(&mut target_clip, target_width, target_height);
            vector.x -= target_clip.x;
            vector.y -= target_clip.y;

            target_x = root_x + vector.x;
            target_y = root_y + vector.y;

            let vector_x = target_x - root_x;
            let vector_y = target_y - root_y;
            edge_length += (vector_x * vector_x + vector_y * vector_y).sqrt();
        }
        edge_length
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

fn node_size(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
}
