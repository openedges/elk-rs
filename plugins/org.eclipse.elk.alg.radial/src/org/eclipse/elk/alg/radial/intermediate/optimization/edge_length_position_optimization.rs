use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::optimization::IEvaluation;

#[derive(Default)]
pub struct EdgeLengthPositionOptimization;

impl IEvaluation for EdgeLengthPositionOptimization {
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

            let (target_x, target_y) = node_center(&target);

            let position = {
                let mut target_mut = target.borrow_mut();
                target_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(CoreOptions::POSITION)
            }
            .unwrap_or_else(KVector::new);

            let (root_x, root_y, root_width, root_height) = {
                let mut root_mut = root.borrow_mut();
                let shape = root_mut.connectable().shape();
                (shape.x(), shape.y(), shape.width(), shape.height())
            };
            let root_x = root_x + position.x + root_width / 2.0;
            let root_y = root_y + position.y + root_height;

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
