use org_eclipse_elk_core::org::eclipse::elk::core::math::{CubicSplineInterpolator, KVector};

#[test]
fn calculate_closed_bezier_spline_test() {
    let v0 = KVector::with_values(5.0, 50.0);
    let v1 = KVector::with_values(10.0, 70.0);
    let v2 = KVector::with_values(30.0, 100.0);
    let v3 = KVector::with_values(60.0, 70.0);
    let v4 = KVector::with_values(70.0, 40.0);
    let vectors = [v0, v1, v2, v3, v4];
    let spline = CubicSplineInterpolator.calculate_closed_bezier_spline(&vectors);
    assert_eq!(v0, spline.get_end_point());

    let v0 = KVector::with_values(5.0, 50.0);
    let v1 = KVector::with_values(10.0, 50.0);
    let v2 = KVector::with_values(30.0, 50.0);
    let v3 = KVector::with_values(60.0, 50.0);
    let v4 = KVector::with_values(70.0, 50.0);
    let vectors_h = [v0, v1, v2, v3, v4];
    let spline = CubicSplineInterpolator.calculate_closed_bezier_spline(&vectors_h);
    assert!((spline.get_start_point().y - 50.0).abs() < 1e-9);
    assert!((spline.get_end_point().y - 50.0).abs() < 1e-9);
    for v in spline.get_inner_points() {
        assert!((v.y - 50.0).abs() < 1e-9);
    }

    let v0 = KVector::with_values(100.0, 10.0);
    let v1 = KVector::with_values(100.0, 50.0);
    let v2 = KVector::with_values(100.0, 100.0);
    let v3 = KVector::with_values(100.0, 120.0);
    let v4 = KVector::with_values(100.0, 200.0);
    let vectors_v = [v0, v1, v2, v3, v4];
    let spline = CubicSplineInterpolator.calculate_closed_bezier_spline(&vectors_v);
    assert!((spline.get_start_point().x - 100.0).abs() < 1e-9);
    assert!((spline.get_end_point().x - 100.0).abs() < 1e-9);
    for v in spline.get_inner_points() {
        assert!((v.x - 100.0).abs() < 1e-9);
    }
}
