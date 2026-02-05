use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::node::Node;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::utils::Utils;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMath, ElkRectangle, KVector};

fn fuzzy_eq(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6
}

fn fuzzy_ge(a: f64, b: f64) -> bool {
    a + 1e-6 >= b
}

#[test]
fn overlap_test() {
    let r1 = ElkRectangle::with_values(0.0, 0.0, 40.0, 80.0);
    let rectangles = vec![
        ElkRectangle::with_values(0.0, 0.0, 70.0, 20.0),
        ElkRectangle::with_values(10.0, 50.0, 70.0, 20.0),
        ElkRectangle::with_values(-40.0, 30.0, 70.0, 20.0),
        ElkRectangle::with_values(-60.0, 70.0, 70.0, 20.0),
        ElkRectangle::with_values(-10.0, 70.0, 70.0, 20.0),
        ElkRectangle::with_values(-20.0, -10.0, 70.0, 20.0),
        ElkRectangle::with_values(10.0, 20.0, 20.0, 20.0),
        ElkRectangle::with_values(-20.0, -20.0, 100.0, 120.0),
        ElkRectangle::with_values(0.0, -0.001, 40.0, 80.0),
    ];

    for r2 in rectangles {
        assert!(test_overlap_computation(&r1, &r2));
    }
}

fn test_overlap_computation(r1: &ElkRectangle, r2: &ElkRectangle) -> bool {
    let overlap = Utils::overlap(r1, r2);
    let c1 = r1.get_center();
    let c2 = r2.get_center();
    let mut d = c2;
    d.sub(&c1);
    let mut c3 = c1;
    d.scale(overlap);
    c3.add(&d);
    let mut r3 = ElkRectangle::from_other(r2);
    let mut delta = c3;
    delta.sub(&c2);
    r3.move_by(&delta);

    fuzzy_ge(ElkMath::shortest_distance(r1, &r3), 0.0)
}

#[test]
fn underlap_test() {
    let r1 = ElkRectangle::with_values(0.0, 0.0, 20.0, 60.0);
    let rectangles = vec![
        ElkRectangle::with_values(40.0, 20.0, 20.0, 20.0),
        ElkRectangle::with_values(40.0, 40.0, 20.0, 20.0),
        ElkRectangle::with_values(30.0, 70.0, 20.0, 20.0),
        ElkRectangle::with_values(20.0, 80.0, 20.0, 20.0),
        ElkRectangle::with_values(10.0, 80.0, 20.0, 20.0),
        ElkRectangle::with_values(0.0, 80.0, 20.0, 20.0),
        ElkRectangle::with_values(-30.0, 70.0, 20.0, 20.0),
        ElkRectangle::with_values(-40.0, 40.0, 20.0, 20.0),
        ElkRectangle::with_values(-40.0, 20.0, 20.0, 20.0),
        ElkRectangle::with_values(-30.0, -20.0, 20.0, 20.0),
        ElkRectangle::with_values(-20.0, -40.0, 20.0, 20.0),
        ElkRectangle::with_values(0.0, -30.0, 20.0, 20.0),
        ElkRectangle::with_values(30.0, -30.0, 20.0, 20.0),
        ElkRectangle::with_values(20.0, 0.0, 20.0, 20.0),
        ElkRectangle::with_values(0.0, 60.0, 20.0, 20.0),
    ];

    for r2 in rectangles {
        assert!(test_underlap_computation(&r1, &r2));
    }
}

fn test_underlap_computation(r1: &ElkRectangle, r2: &ElkRectangle) -> bool {
    let n1 = Node::new(r1.get_center(), ElkRectangle::from_other(r1));
    let mut n2 = Node::new(r2.get_center(), ElkRectangle::from_other(r2));

    let underlap = n1.underlap(&n2);
    let mut direction = n1.vertex;
    direction.sub(&n2.vertex);
    assert!(fuzzy_eq(underlap, n1.distance(&n2, &direction)));

    let mut translation = direction;
    translation.scale_to_length(underlap);
    n2.translate(&translation);
    fuzzy_eq(ElkMath::shortest_distance(&n1.rect, &n2.rect), 0.0)
}

#[test]
fn distance_test() {
    let r1 = ElkRectangle::with_values(0.0, 0.0, 20.0, 60.0);
    let rectangles = vec![
        ElkRectangle::with_values(40.0, 20.0, 20.0, 20.0),
        ElkRectangle::with_values(40.0, 40.0, 20.0, 20.0),
        ElkRectangle::with_values(30.0, 70.0, 20.0, 20.0),
        ElkRectangle::with_values(20.0, 80.0, 20.0, 20.0),
        ElkRectangle::with_values(10.0, 80.0, 20.0, 20.0),
        ElkRectangle::with_values(0.0, 80.0, 20.0, 20.0),
        ElkRectangle::with_values(-30.0, 70.0, 20.0, 20.0),
        ElkRectangle::with_values(-40.0, 40.0, 20.0, 20.0),
        ElkRectangle::with_values(-40.0, 20.0, 20.0, 20.0),
        ElkRectangle::with_values(-30.0, -20.0, 20.0, 20.0),
        ElkRectangle::with_values(-20.0, -40.0, 20.0, 20.0),
        ElkRectangle::with_values(0.0, -30.0, 20.0, 20.0),
        ElkRectangle::with_values(30.0, -30.0, 20.0, 20.0),
        ElkRectangle::with_values(20.0, 0.0, 20.0, 20.0),
        ElkRectangle::with_values(0.0, 60.0, 20.0, 20.0),
    ];

    let vectors = vec![
        (KVector::with_values(-20.0, 20.0), true),
        (KVector::with_values(-80.0, 0.0), false),
        (KVector::with_values(-20.0, 9.0), false),
        (KVector::with_values(0.0, 50.0), false),
        (KVector::with_values(-9.99, 50.0), false),
        (KVector::with_values(60.0, 60.0), false),
        (KVector::with_values(-30.0, 50.0), true),
        (KVector::with_values(-20.0, 130.0), true),
        (KVector::with_values(-20.0, -21.0), false),
    ];

    let n1 = Node::new(r1.get_center(), ElkRectangle::from_other(&r1));
    for (vec, should_collide) in vectors {
        let mut n2 = Node::new(rectangles[12].get_center(), ElkRectangle::from_other(&rectangles[12]));
        let distance = n1.distance(&n2, &vec);
        if should_collide {
            let mut translation = vec;
            translation.scale_to_length(distance);
            n2.translate(&translation);
            assert!(fuzzy_eq(ElkMath::shortest_distance(&n1.rect, &n2.rect), 0.0));
        } else {
            assert!(distance.is_infinite());
        }
    }
}
