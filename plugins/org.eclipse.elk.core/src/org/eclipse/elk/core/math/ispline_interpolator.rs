use std::collections::LinkedList;

use crate::org::eclipse::elk::core::math::bezier_spline::BezierSpline;
use crate::org::eclipse::elk::core::math::kvector::KVector;

pub trait ISplineInterpolator {
    fn interpolate_points(&self, points: &[KVector]) -> BezierSpline;

    fn interpolate_points_with_tangents(
        &self,
        points: &[KVector],
        start_vec: &KVector,
        end_vec: &KVector,
        tangent_scale: bool,
    ) -> BezierSpline;

    fn interpolate_points_list(&self, points: &LinkedList<KVector>) -> BezierSpline;

    fn interpolate_points_list_with_tangents(
        &self,
        points: &LinkedList<KVector>,
        start_vec: &KVector,
        end_vec: &KVector,
        tangent_scale: bool,
    ) -> BezierSpline;
}

