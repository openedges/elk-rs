use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkRectangle;

use crate::org::eclipse::elk::alg::common::t_edge::TEdge;

pub struct Utils;

impl Utils {
    pub fn overlap(r1: &ElkRectangle, r2: &ElkRectangle) -> f64 {
        let horizontal_overlap = (r1.x - (r2.x + r2.width))
            .abs()
            .min((r1.x + r1.width - r2.x).abs());
        let vertical_overlap = (r1.y - (r2.y + r2.height))
            .abs()
            .min((r1.y + r1.height - r2.y).abs());
        let horizontal_center_distance = ((r1.x + r1.width / 2.0) - (r2.x + r2.width / 2.0)).abs();
        if horizontal_center_distance > r1.width / 2.0 + r2.width / 2.0 {
            return 1.0;
        }
        let vertical_center_distance = ((r1.y + r1.height / 2.0) - (r2.y + r2.height / 2.0)).abs();
        if vertical_center_distance > r1.height / 2.0 + r2.height / 2.0 {
            return 1.0;
        }
        if horizontal_center_distance == 0.0 && vertical_center_distance == 0.0 {
            return 0.0;
        }
        if horizontal_center_distance == 0.0 {
            return vertical_overlap / vertical_center_distance + 1.0;
        }
        if vertical_center_distance == 0.0 {
            return horizontal_overlap / horizontal_center_distance + 1.0;
        }
        (horizontal_overlap / horizontal_center_distance)
            .min(vertical_overlap / vertical_center_distance)
            + 1.0
    }

    pub fn get_rect_edges(r: &ElkRectangle) -> Vec<TEdge> {
        vec![
            TEdge::new(r.get_top_left(), r.get_top_right()),
            TEdge::new(r.get_top_left(), r.get_bottom_left()),
            TEdge::new(r.get_bottom_right(), r.get_top_right()),
            TEdge::new(r.get_bottom_right(), r.get_bottom_left()),
        ]
    }
}
