use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::PolylineSelfLoopRouter;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;

fn chain(points: &[(f64, f64)]) -> KVectorChain {
    let mut out = KVectorChain::new();
    for (x, y) in points {
        out.add_vector(KVector::with_values(*x, *y));
    }
    out
}

#[test]
fn test_usual_case() {
    let input = chain(&[
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0, 100.0),
        (-100.0, 100.0),
        (-100.0, -100.0),
        (0.0, -100.0),
    ]);

    let expected = chain(&[
        (90.0, 0.0),
        (100.0, 10.0),
        (100.0, 90.0),
        (90.0, 100.0),
        (-90.0, 100.0),
        (-100.0, 90.0),
        (-100.0, -90.0),
        (-90.0, -100.0),
    ]);

    let router = PolylineSelfLoopRouter;
    assert_eq!(expected, router.cut_corners(&input, 10.0));
}

#[test]
fn test_smaller_distance() {
    let input = chain(&[
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0, 100.0),
        (-100.0, 100.0),
        (-100.0, -100.0),
        (0.0, -100.0),
    ]);

    let expected = chain(&[
        (50.0, 0.0),
        (100.0, 50.0),
        (100.0, 50.0),
        (50.0, 100.0),
        (-20.0, 100.0),
        (-100.0, 20.0),
        (-100.0, -50.0),
        (-50.0, -100.0),
    ]);

    let router = PolylineSelfLoopRouter;
    assert_eq!(expected, router.cut_corners(&input, 80.0));
}
