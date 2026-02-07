use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;

#[derive(Default)]
pub struct PolylineSelfLoopRouter;

impl PolylineSelfLoopRouter {
    const TOLERANCE: f64 = 0.01;

    pub fn cut_corners(&self, bend_points: &KVectorChain, distance: f64) -> KVectorChain {
        assert!(bend_points.len() > 2, "need more than two bend points");

        let mut result = KVectorChain::new();
        let points = bend_points.to_array();

        for window in points.windows(3) {
            let previous = window[0];
            let corner = window[1];
            let next = window[2];

            assert!(
                Self::are_orthogonally_routed(&previous, &corner, &next),
                "bend points must be orthogonally routed"
            );

            let mut offset1 = Self::near_zero_to_zero(KVector::diff(&previous, &corner));
            let mut offset2 = Self::near_zero_to_zero(KVector::diff(&next, &corner));

            let mut effective_distance = distance;
            effective_distance = effective_distance.min((offset1.x + offset1.y).abs() / 2.0);
            effective_distance = effective_distance.min((offset2.x + offset2.y).abs() / 2.0);

            offset1.x = Self::java_signum(offset1.x) * effective_distance;
            offset1.y = Self::java_signum(offset1.y) * effective_distance;
            offset2.x = Self::java_signum(offset2.x) * effective_distance;
            offset2.y = Self::java_signum(offset2.y) * effective_distance;

            let mut first = KVector::from_vector(&corner);
            first.add(&offset1);
            result.add_vector(first);

            let mut second = KVector::from_vector(&corner);
            second.add(&offset2);
            result.add_vector(second);
        }

        result
    }

    fn are_orthogonally_routed(previous: &KVector, corner: &KVector, next: &KVector) -> bool {
        let vertical_horizontal =
            (previous.x - corner.x).abs() <= Self::TOLERANCE && (corner.y - next.y).abs() <= Self::TOLERANCE;
        let horizontal_vertical =
            (previous.y - corner.y).abs() <= Self::TOLERANCE && (corner.x - next.x).abs() <= Self::TOLERANCE;

        vertical_horizontal || horizontal_vertical
    }

    fn near_zero_to_zero(mut vector: KVector) -> KVector {
        if vector.x >= -Self::TOLERANCE && vector.x <= Self::TOLERANCE {
            vector.x = 0.0;
        }
        if vector.y >= -Self::TOLERANCE && vector.y <= Self::TOLERANCE {
            vector.y = 0.0;
        }
        vector
    }

    fn java_signum(value: f64) -> f64 {
        if value > 0.0 {
            1.0
        } else if value < 0.0 {
            -1.0
        } else {
            0.0
        }
    }
}
