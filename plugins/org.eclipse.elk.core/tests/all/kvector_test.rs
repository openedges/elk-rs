use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;

#[test]
fn test_equals() {
    let kvector1 = KVector::with_values(10.0, 10.0);
    let kvector2 = KVector::with_values(10.0, 10.0);
    assert_eq!(kvector1, kvector2);

    assert_ne!(kvector1.x, 0.0); // sanity; Rust equality is typed
}

#[test]
fn test_add_and_sub() {
    let mut kvector1 = KVector::with_values(12.0, 70.0);
    let kvector2 = KVector::with_values(15.0, 17.0);
    let expected = kvector1;
    kvector1.add(&kvector2).sub(&kvector2);
    assert_eq!(expected, kvector1);
}

#[test]
fn test_scale() {
    let mut a = KVector::with_values(12.0, 70.0);
    let mut b = KVector::with_values(12.0, 70.0);

    let a_temp = KVector::with_values(12.0, 70.0);

    a.add(&a_temp).add(&a_temp);
    b.scale(3.0);

    assert_eq!(a, b);
}

#[test]
fn test_translate() {
    let mut v = KVector::with_values(10.0, 30.0);
    v.add_values(40.0, 20.0);
    let b = KVector::with_values(50.0, 50.0);
    assert_eq!(b, v);
}

#[test]
fn test_normalize() {
    let mut v = KVector::with_values(2.0, 0.0);
    let n = KVector::with_values(1.0, 0.0);
    v.normalize();
    assert_eq!(n, v);

    let mut v = KVector::with_values(0.0, 2.0);
    let n = KVector::with_values(0.0, 1.0);
    v.normalize();
    assert_eq!(n, v);
}

#[test]
fn test_to_degrees() {
    let v = KVector::with_values(10.0, 0.0);
    assert!((v.to_degrees() - 0.0).abs() < 0.00001);

    let v = KVector::with_values(10.0, 10.0);
    assert!((v.to_degrees() - 45.0).abs() < 0.00001);

    let v = KVector::with_values(0.0, 10.0);
    assert!((v.to_degrees() - 90.0).abs() < 0.00001);

    let v = KVector::with_values(-10.0, 10.0);
    assert!((v.to_degrees() - 135.0).abs() < 0.00001);

    let v = KVector::with_values(-10.0, 0.0);
    assert!((v.to_degrees() - 180.0).abs() < 0.00001);

    let v = KVector::with_values(-10.0, -10.0);
    assert!((v.to_degrees() - 225.0).abs() < 0.00001);

    let v = KVector::with_values(0.0, -10.0);
    assert!((v.to_degrees() - 270.0).abs() < 0.00001);

    let v = KVector::with_values(10.0, -10.0);
    assert!((v.to_degrees() - 315.0).abs() < 0.00001);
}

#[test]
fn test_distance() {
    let v1 = KVector::with_values(5.0, 50.0);
    let v2 = KVector::with_values(5.0, 50.0);
    assert_eq!(0.0, v1.distance(&v2));

    let v1 = KVector::with_values(0.0, 20.0);
    let v2 = KVector::with_values(0.0, 50.0);
    assert_eq!(30.0, v1.distance(&v2));
}

#[test]
fn test_parse() {
    let v1 = KVector::with_values(5.0, 50.0);
    let mut v2 = KVector::new();
    v2.parse("(5,50)");
    v2.parse("{5,50}");
    assert_eq!(v1, v2);
    v2.parse("[5,50]");
    assert_eq!(v1, v2);
    v2.parse("{(5,50)}");
    assert_eq!(v1, v2);
    v2.parse("[(5,50)]");
    assert_eq!(v1, v2);
    v2.parse("[{5,50}]");
    assert_eq!(v1, v2);
}

#[test]
fn test_apply_bounds() {
    let mut v = KVector::with_values(30.0, 30.0);
    let original = v;
    let lower = KVector::with_values(10.0, 10.0);
    let upper = KVector::with_values(40.0, 40.0);
    v.bound(lower.x, lower.y, upper.x, upper.y);
    assert_eq!(original, v);

    let mut v = KVector::with_values(30.0, 30.0);
    let lower = KVector::with_values(40.0, 40.0);
    let upper = KVector::with_values(60.0, 60.0);
    v.bound(lower.x, lower.y, upper.x, upper.y);
    assert_eq!(lower, v);
}
