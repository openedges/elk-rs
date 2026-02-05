use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, LPortRef};

pub struct SplinesMath;

impl SplinesMath {
    pub const EPSILON: f64 = 0.00000001;
    pub const HALF_PI: f64 = std::f64::consts::PI / 2.0;
    pub const QUATER_PI: f64 = Self::HALF_PI / 2.0;
    pub const THREE_HALF_PI: f64 = Self::HALF_PI + Self::HALF_PI + Self::HALF_PI;
    pub const TWO_PI: f64 = 2.0 * std::f64::consts::PI;
    pub const THREE: f64 = 3.0;

    pub fn intersect_dir(
        pt1: &KVector,
        pt2: &KVector,
        dir_pt1: f64,
        dir_pt2: f64,
    ) -> Option<KVector> {
        let mut pt1_dir = KVector::from_angle(dir_pt1);
        pt1_dir.add(pt1);
        let mut pt2_dir = KVector::from_angle(dir_pt2);
        pt2_dir.add(pt2);
        Self::intersect(pt1, &pt1_dir, pt2, &pt2_dir)
    }

    pub fn intersect(pt1: &KVector, pt2: &KVector, pt3: &KVector, pt4: &KVector) -> Option<KVector> {
        let x1 = pt1.x;
        let y1 = pt1.y;
        let x2 = pt2.x;
        let y2 = pt2.y;
        let x3 = pt3.x;
        let y3 = pt3.y;
        let x4 = pt4.x;
        let y4 = pt4.y;

        let divisor = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
        if divisor.abs() < Self::EPSILON {
            return None;
        }

        let first = x1 * y2 - y1 * x2;
        let second = x3 * y4 - y3 * x4;
        let new_x = (first * (x3 - x4) - second * (x1 - x2)) / divisor;
        let new_y = (first * (y3 - y4) - second * (y1 - y2)) / divisor;
        Some(KVector::with_values(new_x, new_y))
    }

    pub fn inner_angle_vec(vec1: &KVector, vec2: &KVector) -> f64 {
        ((vec1.x * vec2.x + vec1.y * vec2.y) / (vec1.length() * vec2.length())).acos()
    }

    pub fn inner_angle(dir1: f64, dir2: f64) -> f64 {
        let retval = (dir1 - dir2).abs();
        retval % std::f64::consts::PI
    }

    pub fn convert_kvector_to_string(vector: Option<&KVector>) -> String {
        let Some(vector) = vector else {
            return "(null)".to_owned();
        };
        format!("({:.1},{:.1})", vector.x, vector.y)
    }

    pub fn convert_kvector_slice_to_string(list: &[KVector]) -> String {
        if list.is_empty() {
            return "(null)".to_owned();
        }
        let mut parts = Vec::with_capacity(list.len());
        for vector in list {
            parts.push(Self::convert_kvector_to_string(Some(vector)));
        }
        parts.join(", ")
    }

    pub fn length_to_orthogonal(direction: f64, point: &KVector) -> f64 {
        let mut angle = Self::inner_angle(direction, point.to_radians());
        let mut factor = 1.0;
        if angle > Self::HALF_PI {
            factor = -1.0;
            angle -= Self::QUATER_PI;
        }
        factor * angle.cos() * point.length()
    }

    pub fn port_side_to_direction(side: PortSide) -> f64 {
        match side {
            PortSide::North => Self::THREE_HALF_PI,
            PortSide::East => 0.0,
            PortSide::South => Self::HALF_PI,
            PortSide::West => std::f64::consts::PI,
            _ => 0.0,
        }
    }

    pub fn dist_port_to_node_edge(port: &LPortRef, side: PortSide) -> f64 {
        let Ok(mut port_guard) = port.lock() else {
            return 0.0;
        };
        let node = port_guard.node();
        let node_size = node
            .and_then(|node| node.lock().ok().map(|mut n| *n.shape().size_ref()))
            .unwrap_or_default();
        let mut port_pos = *port_guard.shape().position_ref();
        let anchor = *port_guard.anchor_ref();
        port_pos.add(&anchor);

        match side {
            PortSide::North => -port_pos.y,
            PortSide::East => -port_pos.x + node_size.x,
            PortSide::South => -port_pos.y + node_size.y,
            PortSide::West => -port_pos.x,
            _ => 0.0,
        }
    }

    pub fn is_between_int(value: i32, boundary0: i32, boundary1: i32) -> bool {
        if value < boundary0 {
            boundary1 <= value
        } else {
            value <= boundary1 || value == boundary0
        }
    }

    pub fn is_between(value: f64, boundary0: f64, boundary1: f64) -> bool {
        if (boundary0 - value).abs() < Self::EPSILON || (boundary1 - value).abs() < Self::EPSILON {
            return true;
        }
        if (boundary0 - value) > Self::EPSILON {
            (value - boundary1) > Self::EPSILON
        } else {
            (boundary1 - value) > Self::EPSILON
        }
    }

    pub fn get_margin_on_port_side(node: &LNodeRef, side: PortSide) -> f64 {
        let Ok(mut node_guard) = node.lock() else {
            return 0.0;
        };
        let margin = node_guard.margin();
        match side {
            PortSide::North => margin.top,
            PortSide::East => margin.right,
            PortSide::South => margin.bottom,
            PortSide::West => margin.left,
            _ => 0.0,
        }
    }
}
