use org_eclipse_elk_core::org::eclipse::elk::core::math::{BezierSpline, KVector};

fn init_bezier_curve_horizontal() -> (BezierSpline, KVector, KVector, KVector, KVector) {
    let start = KVector::with_values(0.0, 50.0);
    let first = KVector::with_values(10.0, 50.0);
    let second = KVector::with_values(20.0, 50.0);
    let end = KVector::with_values(30.0, 50.0);
    let mut spline = BezierSpline::new();
    spline.add_curve_points(start, first, second, end);
    (spline, start, first, second, end)
}

fn init_bezier_curve_vertical() -> (BezierSpline, KVector, KVector, KVector, KVector) {
    let start = KVector::with_values(50.0, 0.0);
    let first = KVector::with_values(50.0, 20.0);
    let second = KVector::with_values(50.0, 40.0);
    let end = KVector::with_values(50.0, 50.0);
    let mut spline = BezierSpline::new();
    spline.add_curve_points(start, first, second, end);
    (spline, start, first, second, end)
}

#[test]
fn test_get_start_point() {
    let (spline, start, _, _, _) = init_bezier_curve_vertical();
    let v = spline.get_start_point();
    assert!((start.x - v.x).abs() < 1e-9);
    assert!((start.y - v.y).abs() < 1e-9);

    let (spline, start, _, _, _) = init_bezier_curve_horizontal();
    let v = spline.get_start_point();
    assert!((start.x - v.x).abs() < 1e-9);
    assert!((start.y - v.y).abs() < 1e-9);
}

#[test]
fn test_get_end_point() {
    let (spline, _, _, _, end) = init_bezier_curve_vertical();
    let v = spline.get_end_point();
    assert!((end.x - v.x).abs() < 1e-9);
    assert!((end.y - v.y).abs() < 1e-9);

    let (spline, _, _, _, end) = init_bezier_curve_horizontal();
    let v = spline.get_end_point();
    assert!((end.x - v.x).abs() < 1e-9);
    assert!((end.y - v.y).abs() < 1e-9);
}

#[test]
fn test_get_inner_points() {
    let (spline, start, _, _, _) = init_bezier_curve_vertical();
    let vectors = spline.get_inner_points();
    for v in vectors {
        assert!((start.x - v.x).abs() < 1e-8);
    }

    let (spline, start, _, _, _) = init_bezier_curve_horizontal();
    let vectors = spline.get_inner_points();
    for v in vectors {
        assert!((start.y - v.y).abs() < 1e-8);
    }
}

#[test]
fn test_get_base_points() {
    let (spline, start, _, _, _) = init_bezier_curve_vertical();
    let vectors = spline.get_base_points();
    for v in vectors {
        assert!((start.x - v.x).abs() < 1e-8);
    }

    let (spline, start, _, _, _) = init_bezier_curve_horizontal();
    let vectors = spline.get_base_points();
    for v in vectors {
        assert!((start.y - v.y).abs() < 1e-8);
    }
}

#[test]
fn test_get_polyline_apprx() {
    let (spline, start, _, _, _) = init_bezier_curve_vertical();
    let vectors = spline.get_polyline_apprx(50);
    for v in vectors {
        assert!((start.x - v.x).abs() < 1e-8);
    }

    let (spline, start, _, _, _) = init_bezier_curve_horizontal();
    let vectors = spline.get_polyline_apprx(50);
    for v in vectors {
        assert!((start.y - v.y).abs() < 1e-8);
    }
}
