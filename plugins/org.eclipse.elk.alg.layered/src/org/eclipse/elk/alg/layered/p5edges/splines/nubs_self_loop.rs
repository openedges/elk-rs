use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;

use crate::org::eclipse::elk::alg::layered::graph::LPortRef;
use crate::org::eclipse::elk::alg::layered::p5edges::splines::nub_spline::NubSpline;
use crate::org::eclipse::elk::alg::layered::p5edges::splines::splines_math::SplinesMath;

pub struct NubsSelfLoop {
    spline: NubSpline,
    first_label_position: KVector,
}

impl NubsSelfLoop {
    const FRACTION: f64 = 1.3;
    const HALF: f64 = 0.5;
    const DIM: usize = 3;

    pub fn new(dimension: usize, k_vectors: Vec<KVector>) -> Self {
        NubsSelfLoop {
            spline: NubSpline::new_from_vec(true, dimension, k_vectors),
            first_label_position: KVector::new(),
        }
    }

    pub fn new_with_clamped(clamped: bool, dimension: usize, k_vectors: Vec<KVector>) -> Self {
        NubsSelfLoop {
            spline: NubSpline::new_from_vec(clamped, dimension, k_vectors),
            first_label_position: KVector::new(),
        }
    }

    pub fn first_label_position(&self) -> KVector {
        self.first_label_position
    }

    pub fn set_first_label_position(&mut self, position: KVector) {
        self.first_label_position = position;
    }

    pub fn create_side_self_loop(
        source: &LPortRef,
        target: &LPortRef,
        length: f64,
    ) -> NubsSelfLoop {
        let source_pos = port_position(source);
        let target_pos = port_position(target);

        let direction = source
            .lock_ok()
            .map(|port_guard| SplinesMath::port_side_to_direction(port_guard.side()))
            .unwrap_or(0.0);
        let mut first_cp = KVector::from_angle(direction);
        first_cp.scale(length);
        first_cp.add(&source_pos);
        let mut third_cp = KVector::from_angle(direction);
        third_cp.scale(length);
        third_cp.add(&target_pos);
        let mut mid = first_cp;
        mid.sub(&third_cp);
        mid.scale(0.5);
        let mut second_cp = third_cp;
        second_cp.add(&mid);
        let mut mid_dir = KVector::from_angle(direction);
        mid_dir.scale(mid.length());
        second_cp.add(&mid_dir);

        NubsSelfLoop::new(
            Self::DIM,
            vec![source_pos, first_cp, second_cp, third_cp, target_pos],
        )
    }

    pub fn create_corner_self_loop(
        source: &LPortRef,
        target: &LPortRef,
        source_height: f64,
        target_height: f64,
    ) -> NubsSelfLoop {
        let source_dir = source
            .lock_ok()
            .map(|port_guard| SplinesMath::port_side_to_direction(port_guard.side()))
            .unwrap_or(0.0);
        let target_dir = target
            .lock_ok()
            .map(|port_guard| SplinesMath::port_side_to_direction(port_guard.side()))
            .unwrap_or(0.0);

        let source_pos = port_position(source);
        let target_pos = port_position(target);

        let mut first_cp = KVector::from_angle(source_dir);
        first_cp.scale(Self::FRACTION * source_height);
        first_cp.add(&source_pos);
        let mut third_cp = KVector::from_angle(target_dir);
        third_cp.scale(Self::FRACTION * target_height);
        third_cp.add(&target_pos);

        let mut corner_x = 0.0;
        let mut corner_y = 0.0;
        {
            let port_guard = source.lock();
            match port_guard.side() {
                org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide::West => {
                    corner_x = 2.0 * (source_pos.x - source_height)
                        - Self::HALF * (first_cp.x + third_cp.x);
                }
                org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide::East => {
                    corner_x = 2.0 * (source_pos.x + source_height)
                        - Self::HALF * (first_cp.x + third_cp.x);
                }
                org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide::North => {
                    corner_y = 2.0 * (source_pos.y - source_height)
                        - Self::HALF * (first_cp.y + third_cp.y);
                }
                org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide::South => {
                    corner_y = 2.0 * (source_pos.y + source_height)
                        - Self::HALF * (first_cp.y + third_cp.y);
                }
                _ => {}
            }
        }

        {
            let port_guard = target.lock();
            match port_guard.side() {
                org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide::West => {
                    corner_x = 2.0 * (target_pos.x - target_height)
                        - Self::HALF * (third_cp.x + first_cp.x);
                }
                org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide::East => {
                    corner_x = 2.0 * (target_pos.x + target_height)
                        - Self::HALF * (third_cp.x + first_cp.x);
                }
                org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide::North => {
                    corner_y = 2.0 * (target_pos.y - target_height)
                        - Self::HALF * (third_cp.y + first_cp.y);
                }
                org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide::South => {
                    corner_y = 2.0 * (target_pos.y + target_height)
                        - Self::HALF * (third_cp.y + first_cp.y);
                }
                _ => {}
            }
        }

        let second_cp = KVector::with_values(corner_x, corner_y);

        NubsSelfLoop::new(
            Self::DIM,
            vec![source_pos, first_cp, second_cp, third_cp, target_pos],
        )
    }

    pub fn create_across_self_loop(
        source: &LPortRef,
        target: &LPortRef,
        source_bend_point: KVector,
        corner_bend_points: &[KVector],
        target_bend_point: KVector,
    ) -> NubsSelfLoop {
        let source_pos = port_position(source);
        let target_pos = port_position(target);

        assert!(corner_bend_points.len() == 2);
        let third_cp = corner_bend_points[0];
        let fourth_cp = corner_bend_points[1];

        let mut middle_cp = fourth_cp;
        middle_cp.add(&third_cp);
        middle_cp.scale(Self::HALF);

        NubsSelfLoop::new(
            Self::DIM,
            vec![
                source_pos,
                source_bend_point,
                third_cp,
                middle_cp,
                fourth_cp,
                target_bend_point,
                target_pos,
            ],
        )
    }

    pub fn create_three_side_self_loop(
        source: &LPortRef,
        target: &LPortRef,
        source_bend_point: KVector,
        corner_bend_points: &[KVector],
        target_bend_point: KVector,
    ) -> NubsSelfLoop {
        let source_pos = port_position(source);
        let target_pos = port_position(target);

        assert!(corner_bend_points.len() == 3);
        let third_cp = corner_bend_points[0];
        let fourth_cp = corner_bend_points[1];
        let fifth_cp = corner_bend_points[2];

        NubsSelfLoop::new(
            Self::DIM,
            vec![
                source_pos,
                source_bend_point,
                third_cp,
                fourth_cp,
                fifth_cp,
                target_bend_point,
                target_pos,
            ],
        )
    }

    pub fn create_four_side_self_loop(
        source: &LPortRef,
        target: &LPortRef,
        source_bend_point: KVector,
        corner_bend_points: &[KVector],
        target_bend_point: KVector,
    ) -> NubsSelfLoop {
        let source_pos = port_position(source);
        let target_pos = port_position(target);

        assert!(corner_bend_points.len() == 4);
        let third_cp = corner_bend_points[0];
        let fourth_cp = corner_bend_points[1];
        let fifth_cp = corner_bend_points[2];
        let sixth_cp = corner_bend_points[3];

        NubsSelfLoop::new(
            Self::DIM,
            vec![
                source_pos,
                source_bend_point,
                third_cp,
                fourth_cp,
                fifth_cp,
                sixth_cp,
                target_bend_point,
                target_pos,
            ],
        )
    }
}

impl std::ops::Deref for NubsSelfLoop {
    type Target = NubSpline;

    fn deref(&self) -> &Self::Target {
        &self.spline
    }
}

impl std::ops::DerefMut for NubsSelfLoop {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.spline
    }
}

fn port_position(port: &LPortRef) -> KVector {
    let mut port_guard = port.lock();
    let mut pos = *port_guard.shape().position_ref();
    pos.add(port_guard.anchor_ref());
    pos
}
