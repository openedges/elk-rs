use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::optimization::IEvaluation;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;

#[derive(Default)]
pub struct CrossingMinimizationPosition;

/// Pre-extracted node data for crossing check: (center_x, center_y, position_x, position_y)
type NodeCrossingData = (f64, f64, f64, f64);

impl CrossingMinimizationPosition {
    /// Check crossing using pre-extracted flat data (zero borrows).
    fn is_crossing_soa(d1: &NodeCrossingData, d2: &NodeCrossingData) -> bool {
        let (cx1, cy1, px1, py1) = *d1;
        let (cx2, cy2, px2, py2) = *d2;

        let m1 = (cy1 - py1) / (cx1 - px1);
        let b1 = cy1 - m1 * cx1;

        let m2 = (cy2 - py2) / (cx2 - px2);
        let b2 = cy2 - m2 * cx2;

        let x_cut = (b1 - b2) / (m2 - m1);
        if (px1 < x_cut && cx1 < x_cut) || (x_cut < px1 && x_cut < cx1) {
            return false;
        }
        if (px2 < x_cut && cx2 < x_cut) || (x_cut < px2 && x_cut < cx2) {
            return false;
        }
        true
    }
}

impl IEvaluation for CrossingMinimizationPosition {
    fn evaluate(&self, root_node: &ElkNodeRef) -> f64 {
        let _ = self;

        // Pre-extract root center ONCE (note: original uses x() for both components — parity)
        let (root_x, root_y) = {
            let mut root_mut = root_node.borrow_mut();
            let shape = root_mut.connectable().shape();
            (
                shape.x() + shape.width() / 2.0,
                shape.x() + shape.width() / 2.0,
            )
        };

        let nodes = RadialUtil::get_successors(root_node);

        // Pre-extract all node centers + positions: O(n) borrows instead of O(n²)
        let node_data: Vec<NodeCrossingData> = nodes
            .iter()
            .map(|node| {
                let (cx, cy) = {
                    let mut nm = node.borrow_mut();
                    let shape = nm.connectable().shape();
                    (
                        shape.x() + shape.width() / 2.0,
                        shape.y() + shape.height() / 2.0,
                    )
                };
                let pos = {
                    let mut nm = node.borrow_mut();
                    nm.connectable()
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .get_property(CoreOptions::POSITION)
                }
                .unwrap_or_else(KVector::new);
                (cx, cy, pos.x + root_x, pos.y + root_y)
            })
            .collect();

        // O(n²) crossing check with zero borrows
        let mut crossings = 0;
        for i in 0..node_data.len() {
            for j in (i + 1)..node_data.len() {
                if Self::is_crossing_soa(&node_data[i], &node_data[j]) {
                    crossings += 1;
                }
            }
        }
        crossings as f64
    }
}
