use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::{Point, RectilinearConvexHull};
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkRectangle;

#[test]
fn test_cross() {
    let points = vec![
        p(0.0, 1.0),
        p(1.0, 1.0),
        p(1.0, 0.0),
        p(2.0, 0.0),
        p(2.0, 1.0),
        p(3.0, 1.0),
        p(3.0, 2.0),
        p(2.0, 2.0),
        p(2.0, 3.0),
        p(1.0, 3.0),
        p(1.0, 2.0),
        p(0.0, 2.0),
    ];

    let hull = RectilinearConvexHull::of(points.clone());
    assert_eq!(hull.get_hull(), &points);

    let expected_rects = vec![
        ElkRectangle::with_values(0.0, 1.0, 1.0, 1.0),
        ElkRectangle::with_values(1.0, 0.0, 1.0, 3.0),
        ElkRectangle::with_values(2.0, 1.0, 1.0, 1.0),
    ];
    let actual_rects = hull.split_into_rectangles();
    assert_eq!(actual_rects, expected_rects);
}

#[test]
fn test_little_robot() {
    let points = vec![
        p(0.0, 2.0),
        p(1.0, 2.0),
        p(1.0, 1.0),
        p(2.0, 1.0),
        p(2.0, 0.0),
        p(2.0, 0.0),
        p(2.0, 1.0),
        p(3.0, 1.0),
        p(3.0, 2.0),
        p(4.0, 2.0),
        p(4.0, 2.0),
        p(3.0, 2.0),
        p(3.0, 3.0),
        p(2.0, 3.0),
        p(2.0, 4.0),
        p(2.0, 4.0),
        p(2.0, 3.0),
        p(1.0, 3.0),
        p(1.0, 2.0),
        p(0.0, 2.0),
    ];

    let hull = RectilinearConvexHull::of(points.clone());
    assert_eq!(hull.get_hull(), &points);

    let expected_rects = vec![
        ElkRectangle::with_values(0.0, 2.0, 1.0, 0.0),
        ElkRectangle::with_values(1.0, 1.0, 1.0, 2.0),
        ElkRectangle::with_values(2.0, 0.0, 0.0, 4.0),
        ElkRectangle::with_values(2.0, 1.0, 1.0, 2.0),
        ElkRectangle::with_values(3.0, 2.0, 1.0, 0.0),
    ];
    let actual_rects = hull.split_into_rectangles();
    assert_eq!(actual_rects, expected_rects);
}

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}
