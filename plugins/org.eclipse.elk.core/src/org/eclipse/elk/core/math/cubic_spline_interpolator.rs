use std::collections::LinkedList;

use crate::org::eclipse::elk::core::math::bezier_spline::BezierSpline;
use crate::org::eclipse::elk::core::math::ispline_interpolator::ISplineInterpolator;
use crate::org::eclipse::elk::core::math::kvector::KVector;

#[derive(Clone, Debug, Default)]
pub struct CubicSplineInterpolator;

impl CubicSplineInterpolator {
    const INTERP_COEF_EVEN: [[f64; 7]; 7] = [
        [0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.2677, -0.0667, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.2679, -0.0714, 0.0179, 0.0, 0.0, 0.0, 0.0],
        [0.2679, -0.0718, 0.0191, -0.0048, 0.0, 0.0, 0.0],
        [0.2679, -0.0718, 0.0192, -0.0051, 0.0013, 0.0, 0.0],
        [0.2679, -0.0718, 0.0192, -0.0052, 0.0014, -0.0003, 0.0],
        [0.2679, -0.0718, 0.0192, -0.0052, 0.0014, -0.0004, 0.0001],
    ];

    const INTERP_COEF_ODD: [[f64; 7]; 7] = [
        [0.3333, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.2727, -0.0909, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.2683, -0.0732, 0.0244, 0.0, 0.0, 0.0, 0.0],
        [0.2680, -0.0719, 0.0196, -0.0065, 0.0, 0.0, 0.0],
        [0.2680, -0.0718, 0.0193, -0.0053, 0.0018, 0.0, 0.0],
        [0.2679, -0.0718, 0.0192, -0.0052, 0.0014, -0.0005, 0.0],
        [0.2679, -0.0718, 0.0192, -0.0052, 0.0014, -0.0004, 0.0001],
    ];

    const MAX_K: usize = 7;
    const TANGENT_SCALE: f64 = 0.25;

    pub fn calculate_closed_bezier_spline(&self, points: &[KVector]) -> BezierSpline {
        let mut spline = BezierSpline::new();
        let n = points.len();
        if n == 0 {
            return spline;
        }
        let even = n.is_multiple_of(2);
        let mut m = if even { (n - 2) / 2 } else { (n - 1) / 2 };
        if m > Self::MAX_K {
            m = Self::MAX_K;
        }

        let mut d = vec![KVector::new(); n];
        for i in 0..n {
            let mut di = KVector::new();
            for k in 1..=m {
                let a = if even {
                    Self::INTERP_COEF_ODD[m - 1][k - 1]
                } else {
                    Self::INTERP_COEF_EVEN[m - 1][k - 1]
                };
                let pk = points[(i + k) % n];
                let mk = points[(i + n - k) % n];
                di.x += a * (pk.x - mk.x);
                di.y += a * (pk.y - mk.y);
            }
            d[i] = di;
        }

        for i in 0..n {
            let mut bend1 = points[i];
            bend1.add(&d[i]);
            let mut bend2 = points[(i + 1) % n];
            bend2.sub(&d[(i + 1) % n]);
            spline.add_curve_points(points[i], bend1, bend2, points[(i + 1) % n]);
        }

        spline
    }

    fn calculate_open_bezier_spline_default(&self, points: &[KVector]) -> BezierSpline {
        if points.len() < 2 {
            return BezierSpline::new();
        }
        let mut start_vec = points[1];
        start_vec.sub(&points[0]).normalize();
        let mut end_vec = points[points.len() - 2];
        end_vec.sub(&points[points.len() - 1]).normalize();
        self.calculate_open_bezier_spline(points, &start_vec, &end_vec, false)
    }

    fn calculate_open_bezier_spline(
        &self,
        points: &[KVector],
        start_tan: &KVector,
        end_tan: &KVector,
        tangent_scale: bool,
    ) -> BezierSpline {
        let mut spline = BezierSpline::new();
        if points.len() < 2 {
            return spline;
        }

        let n = points.len() - 1;
        let mut t: Vec<KVector> = vec![KVector::new(); 2 * n];
        let mut d: Vec<KVector> = vec![KVector::new(); n + 1];

        let mut start_scale = 1.0;
        let mut end_scale = 1.0;
        if tangent_scale {
            if points.len() == 2 {
                start_scale = points[0].distance(&points[1]) * Self::TANGENT_SCALE;
                end_scale = start_scale;
            } else {
                start_scale = points[0].distance(&points[1]) * Self::TANGENT_SCALE;
                end_scale = points[n - 1].distance(&points[n]) * Self::TANGENT_SCALE;
            }
        }

        let mut d0 = *start_tan;
        d0.scale(start_scale);
        let mut dn = *end_tan;
        dn.scale(end_scale);
        d[0] = d0;
        d[n] = dn;

        let mut t0 = points[0];
        t0.add(&d[0]);
        let mut tn = points[n];
        tn.sub(&d[n]);
        t[0] = t0;
        t[n] = tn;

        for i in 1..n {
            t[i] = points[i];
            t[2 * n - i] = t[i];
        }

        let m = std::cmp::min(n - 1, Self::MAX_K);
        for i in 1..n {
            let mut di = KVector::new();
            for k in 1..=m {
                let a = Self::INTERP_COEF_EVEN[m - 1][k - 1];
                let tp = t[i + k];
                let tm = t[(i as isize - k as isize).unsigned_abs()];
                di.x += a * (tp.x - tm.x);
                di.y += a * (tp.y - tm.y);
            }
            d[i] = di;
        }

        for i in 0..n {
            let mut bend1 = points[i];
            bend1.add(&d[i]);
            let mut bend2 = points[i + 1];
            bend2.sub(&d[i + 1]);
            spline.add_curve_points(points[i], bend1, bend2, points[i + 1]);
        }

        spline
    }
}

impl ISplineInterpolator for CubicSplineInterpolator {
    fn interpolate_points(&self, points: &[KVector]) -> BezierSpline {
        self.calculate_open_bezier_spline_default(points)
    }

    fn interpolate_points_with_tangents(
        &self,
        points: &[KVector],
        start_vec: &KVector,
        end_vec: &KVector,
        tangent_scale: bool,
    ) -> BezierSpline {
        self.calculate_open_bezier_spline(points, start_vec, end_vec, tangent_scale)
    }

    fn interpolate_points_list(&self, points: &LinkedList<KVector>) -> BezierSpline {
        let vec: Vec<KVector> = points.iter().copied().collect();
        self.calculate_open_bezier_spline_default(&vec)
    }

    fn interpolate_points_list_with_tangents(
        &self,
        points: &LinkedList<KVector>,
        start_vec: &KVector,
        end_vec: &KVector,
        tangent_scale: bool,
    ) -> BezierSpline {
        let vec: Vec<KVector> = points.iter().copied().collect();
        self.calculate_open_bezier_spline(&vec, start_vec, end_vec, tangent_scale)
    }
}
