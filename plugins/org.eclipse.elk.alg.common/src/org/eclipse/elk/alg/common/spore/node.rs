use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMath, ElkRectangle, KVector};

use crate::org::eclipse::elk::alg::common::spore::internal_properties::InternalProperties;
use crate::org::eclipse::elk::alg::common::utils::Utils;

#[derive(Clone, Debug)]
pub struct Node {
    pub original_vertex: KVector,
    pub vertex: KVector,
    pub rect: ElkRectangle,
}

impl Node {
    pub fn new(vertex: KVector, rect: ElkRectangle) -> Self {
        Node {
            original_vertex: vertex,
            vertex,
            rect,
        }
    }

    pub fn translate(&mut self, v: &KVector) {
        self.vertex.add(v);
        self.rect.x += v.x;
        self.rect.y += v.y;
    }

    pub fn set_center_position(&mut self, p: &KVector) {
        let mut delta = *p;
        delta.sub(&self.vertex);
        self.translate(&delta);
    }

    pub fn underlap(&self, other: &Node) -> f64 {
        let horizontal_center_distance =
            (self.rect.get_center().x - other.rect.get_center().x).abs();
        let vertical_center_distance = (self.rect.get_center().y - other.rect.get_center().y).abs();
        let mut h_scale = 1.0;
        let mut v_scale = 1.0;
        if horizontal_center_distance > self.rect.width / 2.0 + other.rect.width / 2.0 {
            let horizontal_underlap = (self.rect.x - (other.rect.x + other.rect.width))
                .abs()
                .min((self.rect.x + self.rect.width - other.rect.x).abs());
            h_scale = 1.0 - horizontal_underlap / horizontal_center_distance;
        }
        if vertical_center_distance > self.rect.height / 2.0 + other.rect.height / 2.0 {
            let vertical_underlap = (self.rect.y - (other.rect.y + other.rect.height))
                .abs()
                .min((self.rect.y + self.rect.height - other.rect.y).abs());
            v_scale = 1.0 - vertical_underlap / vertical_center_distance;
        }
        let scale = h_scale.min(v_scale);
        (1.0 - scale)
            * (horizontal_center_distance * horizontal_center_distance
                + vertical_center_distance * vertical_center_distance)
                .sqrt()
    }

    pub fn distance(&self, other: &Node, v: &KVector) -> f64 {
        let mut result = f64::INFINITY;
        for e1 in Utils::get_rect_edges(&self.rect) {
            for e2 in Utils::get_rect_edges(&other.rect) {
                let distance = ElkMath::distance(&e1.u, &e1.v, &e2.u, &e2.v, v);
                result = result.min(distance);
            }
        }
        result
    }

    pub fn touches(&self, other: &Node) -> bool {
        fuzzy_compare(
            self.rect.x,
            other.rect.x + other.rect.width,
            InternalProperties::FUZZINESS,
        ) <= 0
            && fuzzy_compare(
                other.rect.x,
                self.rect.x + self.rect.width,
                InternalProperties::FUZZINESS,
            ) <= 0
            && fuzzy_compare(
                self.rect.y,
                other.rect.y + other.rect.height,
                InternalProperties::FUZZINESS,
            ) <= 0
            && fuzzy_compare(
                other.rect.y,
                self.rect.y + self.rect.height,
                InternalProperties::FUZZINESS,
            ) <= 0
    }
}

fn fuzzy_compare(a: f64, b: f64, eps: f64) -> i32 {
    if (a - b).abs() <= eps {
        0
    } else if a < b {
        -1
    } else {
        1
    }
}
