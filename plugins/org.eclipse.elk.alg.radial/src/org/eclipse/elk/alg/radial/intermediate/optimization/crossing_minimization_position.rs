use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::optimization::IEvaluation;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;

#[derive(Default)]
pub struct CrossingMinimizationPosition;

impl CrossingMinimizationPosition {
    fn is_crossing(root: &ElkNodeRef, node1: &ElkNodeRef, node2: &ElkNodeRef) -> bool {
        let (root_x, root_y) = {
            let mut root_mut = root.borrow_mut();
            let shape = root_mut.connectable().shape();
            (
                shape.x() + shape.width() / 2.0,
                shape.x() + shape.width() / 2.0,
            )
        };

        let node1_vector = {
            let mut node_mut = node1.borrow_mut();
            let shape = node_mut.connectable().shape();
            KVector::with_values(
                shape.x() + shape.width() / 2.0,
                shape.y() + shape.height() / 2.0,
            )
        };
        let mut position1 = {
            let mut node_mut = node1.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::POSITION)
        }
        .unwrap_or_else(KVector::new);
        position1.x += root_x;
        position1.y += root_y;

        let m1 = (node1_vector.y - position1.y) / (node1_vector.x - position1.x);
        let b1 = node1_vector.y - m1 * node1_vector.x;

        let node2_vector = {
            let mut node_mut = node2.borrow_mut();
            let shape = node_mut.connectable().shape();
            KVector::with_values(
                shape.x() + shape.width() / 2.0,
                shape.y() + shape.height() / 2.0,
            )
        };
        let mut position2 = {
            let mut node_mut = node2.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::POSITION)
        }
        .unwrap_or_else(KVector::new);
        position2.x += root_x;
        position2.y += root_y;

        let m2 = (node2_vector.y - position2.y) / (node2_vector.x - position2.x);
        let b2 = node2_vector.y - m2 * node2_vector.x;

        let x_cut = (b1 - b2) / (m2 - m1);
        if (position1.x < x_cut && node1_vector.x < x_cut)
            || (x_cut < position1.x && x_cut < node1_vector.x)
        {
            return false;
        }
        if (position2.x < x_cut && node2_vector.x < x_cut)
            || (x_cut < position2.x && x_cut < node2_vector.x)
        {
            return false;
        }
        true
    }
}

impl IEvaluation for CrossingMinimizationPosition {
    fn evaluate(&self, root_node: &ElkNodeRef) -> f64 {
        let _ = self;
        let nodes = RadialUtil::get_successors(root_node);
        let mut crossings = 0;
        for (index, node1) in nodes.iter().enumerate() {
            for node2 in nodes.iter().skip(index + 1) {
                if Self::is_crossing(root_node, node1, node2) {
                    crossings += 1;
                }
            }
        }
        crossings as f64
    }
}
