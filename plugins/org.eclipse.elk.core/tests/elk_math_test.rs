use org_eclipse_elk_core::org::eclipse::elk::core::math::{
    ElkMath, ElkRectangle, KVector, KVectorChain,
};

#[test]
fn test_point_contains() {
    let rect = ElkRectangle::with_values(23.0, 14.0, 20.0, 20.0);
    let contained = KVector::with_values(26.0, 19.0);
    assert!(ElkMath::contains((&rect, &contained)));

    let not_contained = KVector::with_values(10.0, 9.0);
    assert!(!ElkMath::contains((&rect, &not_contained)));

    let on_border = KVector::with_values(23.0, 20.0);
    assert!(!ElkMath::contains((&rect, &on_border)));

    let on_corner = KVector::with_values(23.0, 14.0);
    assert!(!ElkMath::contains((&rect, &on_corner)));
}

#[test]
fn test_line_contains() {
    let rect = ElkRectangle::with_values(23.0, 14.0, 20.0, 20.0);
    let line11 = KVector::with_values(24.0, 20.0);
    let line12 = KVector::with_values(40.0, 32.0);
    assert!(ElkMath::contains((&rect, &line11, &line12)));

    let line21 = KVector::with_values(10.0, 10.0);
    let line22 = KVector::with_values(40.0, 32.0);
    assert!(!ElkMath::contains((&rect, &line21, &line22)));
}

#[test]
fn test_path_contains() {
    let rect = ElkRectangle::with_values(23.0, 14.0, 20.0, 20.0);

    let mut path = KVectorChain::new();
    path.add_values(24.0, 15.0);
    path.add_values(27.0, 20.0);
    path.add_values(39.0, 30.0);
    path.add_values(29.0, 19.0);
    assert!(ElkMath::contains((&rect, &path)));

    let mut path2 = path.clone();
    path2.add_values(23.0, 14.0);
    assert!(!ElkMath::contains((&rect, &path2)));

    path.add_values(10.0, 10.0);
    assert!(!ElkMath::contains((&rect, &path)));
}

#[test]
fn test_line_line_intersect() {
    let l11 = KVector::with_values(10.0, 10.0);
    let l12 = KVector::with_values(20.0, 20.0);

    let l21 = KVector::with_values(11.0, 21.0);
    let l22 = KVector::with_values(21.0, 11.0);
    assert!(ElkMath::intersects((&l11, &l12, &l21, &l22)));

    let l21 = KVector::with_values(10.0, 10.0);
    let l22 = KVector::with_values(15.0, 10.0);
    assert!(!ElkMath::intersects((&l11, &l12, &l21, &l22)));

    let l21 = KVector::with_values(1.0, 2.0);
    let l22 = KVector::with_values(2.0, 1.0);
    assert!(!ElkMath::intersects((&l11, &l12, &l21, &l22)));

    assert!(!ElkMath::intersects((&l11, &l12, &l11, &l12)));
    assert!(!ElkMath::intersects((&l11, &l12, &l12, &l11)));

    let l21 = KVector::with_values(11.0, 1.0);
    let l22 = KVector::with_values(21.0, 21.0);
    assert!(!ElkMath::intersects((&l11, &l12, &l21, &l22)));
}

#[test]
fn test_path_intersects() {
    let rect = ElkRectangle::with_values(23.0, 14.0, 20.0, 20.0);

    let mut path = KVectorChain::new();
    path.add_values(24.0, 15.0);
    path.add_values(27.0, 20.0);
    path.add_values(39.0, 30.0);
    path.add_values(29.0, 19.0);
    assert!(!ElkMath::intersects((&rect, &path)));

    let mut path2 = path.clone();
    path2.add_values(23.0, 14.0);
    assert!(!ElkMath::intersects((&rect, &path2)));

    path.add_values(10.0, 10.0);
    assert!(ElkMath::intersects((&rect, &path)));
}

#[test]
fn test_factl() {
    assert_eq!(1, ElkMath::factl(0));
    assert_eq!(1, ElkMath::factl(1));
    assert_eq!(2432902008176640000, ElkMath::factl(20));
}

#[test]
#[should_panic]
fn test_factl_little_illegal_argument_exception() {
    ElkMath::factl(-50);
}

#[test]
#[should_panic]
fn test_factl_big_illegal_argument_exception() {
    ElkMath::factl(21);
}

#[test]
fn test_factd() {
    assert!((ElkMath::factd(0) - 1.0).abs() < 1e-9);
    assert!((ElkMath::factd(1) - 1.0).abs() < 1e-9);
}

#[test]
#[should_panic]
fn test_factd_little_illegal_argument_exception() {
    ElkMath::factd(-1);
}

#[test]
fn test_binomiall() {
    assert_eq!(1, ElkMath::binomiall(2, 0));
    assert_eq!(1, ElkMath::binomiall(20, 20));
    assert_eq!(2, ElkMath::binomiall(2, 1));
}

#[test]
#[should_panic]
fn test_binomiall_little_illegal_argument_exception() {
    ElkMath::binomiall(-1, 1);
}

#[test]
fn test_binomiald() {
    assert!((ElkMath::binomiald(2, 0) - 1.0).abs() < 1e-9);
    assert!((ElkMath::binomiald(20, 20) - 1.0).abs() < 1e-9);
    assert!((ElkMath::binomiald(2, 1) - 2.0).abs() < 1e-9);
}

#[test]
#[should_panic]
fn test_binomiald_little_illegal_argument_exception() {
    ElkMath::binomiald(-1, 1);
}

#[test]
fn test_pow() {
    let ad = 10.0;
    let af = 10.0f32;
    assert!((ElkMath::powd(ad, 0) - 1.0).abs() < 1e-9);
    assert!((ElkMath::powf(af, 0) - 1.0).abs() < 1e-6);
    assert!((ElkMath::powd(ad, 2) - 100.0).abs() < 1e-9);
    assert!((ElkMath::powf(af, 2) - 100.0).abs() < 1e-6);
}

#[test]
fn test_calc_bezier_points() {
    let kvector1 = KVector::with_values(10.0, 10.0);
    let kvector2 = KVector::with_values(20.0, 20.0);
    let kvector3 = KVector::with_values(30.0, 30.0);
    let kvector4 = KVector::with_values(50.0, 50.0);

    let result = ElkMath::approximate_bezier_segment(20, &[kvector1, kvector2, kvector3, kvector4]);
    let last = result.last().expect("segment should not be empty");
    assert!((kvector4.x - last.x).abs() < 1e-9);
    assert!((kvector4.y - last.y).abs() < 1e-9);

    let kvector1 = KVector::with_values(50.0, 10.0);
    let kvector2 = KVector::with_values(70.0, 10.0);
    let kvector3 = KVector::with_values(80.0, 10.0);
    let kvector4 = KVector::with_values(100.0, 10.0);
    let result = ElkMath::approximate_bezier_segment(20, &[kvector1, kvector2, kvector3, kvector4]);
    for k in result {
        assert!((k.y - 10.0).abs() < 1e-9);
    }
}

#[test]
fn test_approximate_spline() {
    let kvector1 = KVector::with_values(10.0, 10.0);
    let kvector2 = KVector::with_values(20.0, 20.0);
    let kvector3 = KVector::with_values(30.0, 30.0);
    let kvector4 = KVector::with_values(50.0, 50.0);

    let vectors = ElkMath::approximate_bezier_segment(20, &[kvector1, kvector2, kvector3, kvector4]);
    let control_points = KVectorChain::from_vectors(&vectors);
    let result = ElkMath::approximate_bezier_spline(&control_points);
    let last = result.get(result.size() - 1);
    assert!((kvector4.x - last.x).abs() < 1e-9);
    assert!((kvector4.y - last.y).abs() < 1e-9);

    let kvector1 = KVector::with_values(50.0, 10.0);
    let kvector2 = KVector::with_values(70.0, 10.0);
    let kvector3 = KVector::with_values(80.0, 10.0);
    let kvector4 = KVector::with_values(100.0, 10.0);
    let vectors = ElkMath::approximate_bezier_segment(20, &[kvector1, kvector2, kvector3, kvector4]);
    let control_points = KVectorChain::from_vectors(&vectors);
    let result = ElkMath::approximate_bezier_spline(&control_points);
    for kv in result.iter() {
        assert!((kv.y - 10.0).abs() < 1e-9);
    }
}

#[test]
fn test_distance_from_spline() {
    let kvector1 = KVector::with_values(10.0, 10.0);
    let kvector2 = KVector::with_values(20.0, 20.0);
    let kvector3 = KVector::with_values(30.0, 30.0);
    let kvector4 = KVector::with_values(50.0, 50.0);

    let mut needle = kvector4;
    let result = ElkMath::distance_from_bezier_segment(kvector1, kvector2, kvector3, kvector4, needle);
    assert!((result - 0.0).abs() < 0.01);

    needle = kvector3;
    let result = ElkMath::distance_from_bezier_segment(kvector1, kvector2, kvector3, kvector4, needle);
    assert!((result - 0.0).abs() < 0.01);

    needle = kvector2;
    let result = ElkMath::distance_from_bezier_segment(kvector1, kvector2, kvector3, kvector4, needle);
    assert!((result - 0.0).abs() < 0.01);

    needle = kvector1;
    let result = ElkMath::distance_from_bezier_segment(kvector1, kvector2, kvector3, kvector4, needle);
    assert!((result - 0.0).abs() < 0.01);
}

#[test]
fn test_max() {
    assert_eq!(7, ElkMath::maxi(&[1, 7, 5, 6]));
    assert!((ElkMath::maxf(&[1.0, 7.0, 5.0, 6.0]) - 7.0).abs() < 1e-6);
    assert!((ElkMath::maxd(&[1.0, 7.0, 5.0, 6.0]) - 7.0).abs() < 1e-9);
}

#[test]
fn test_min() {
    assert_eq!(1, ElkMath::mini(&[1, 7, 5, 6]));
    assert_eq!(0, ElkMath::mini(&[8, 1, 9, 0]));
    assert_eq!(8, ElkMath::mini(&[8, 8, 8, 8]));
    assert!((ElkMath::minf(&[1.0, 7.0, 5.0, 6.0]) - 1.0).abs() < 1e-6);
    assert!((ElkMath::minf(&[8.0, 1.0, 9.0, 0.0]) - 0.0).abs() < 1e-6);
    assert!((ElkMath::minf(&[8.0, 8.0, 8.0, 8.0]) - 8.0).abs() < 1e-6);
    assert!((ElkMath::mind(&[1.0, 7.0, 5.0, 6.0]) - 1.0).abs() < 1e-9);
    assert!((ElkMath::mind(&[8.0, 1.0, 9.0, 0.0]) - 0.0).abs() < 1e-9);
    assert!((ElkMath::mind(&[8.0, 8.0, 8.0, 8.0]) - 8.0).abs() < 1e-9);
}

#[test]
fn test_average() {
    assert_eq!(4, ElkMath::averagel(&[5, 8, 2, 1]));
    assert_eq!(2, ElkMath::averagel(&[5, 0, 2, 1]));
    assert!((ElkMath::averagef(&[5.0, 8.0, 2.0, 1.0]) - 4.0).abs() < 1e-6);
    assert!((ElkMath::averagef(&[5.0, 0.0, 2.0, 1.0]) - 2.0).abs() < 1e-6);
    assert!((ElkMath::averaged(&[5.0, 8.0, 2.0, 1.0]) - 4.0).abs() < 1e-9);
    assert!((ElkMath::averaged(&[5.0, 0.0, 2.0, 1.0]) - 2.0).abs() < 1e-9);
}
