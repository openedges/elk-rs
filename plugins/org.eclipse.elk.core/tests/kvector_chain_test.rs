use org_eclipse_elk_core::org::eclipse::elk::core::math::{KVector, KVectorChain};

#[test]
fn test_parse() {
    let v0 = KVector::with_values(5.0, 50.0);
    let v1 = KVector::with_values(10.0, 50.0);
    let v2 = KVector::with_values(30.0, 50.0);
    let mut kv = KVectorChain::new();
    kv.parse("{(5,50),(10,50),(30,50)}");

    assert_eq!(v0, kv.get(0));
    assert_eq!(v1, kv.get(1));
    assert_eq!(v2, kv.get(2));

    kv = KVectorChain::new();
    kv.parse("{(5,50),(10,50),(30,)}");
    assert_eq!(v0, kv.get(0));
    assert_eq!(v1, kv.get(1));
    assert_eq!(2, kv.size());

    kv = KVectorChain::new();
    kv.parse("{(5; 50 ], [10 , 50 ),(30,,,) }");
    assert_eq!(v0, kv.get(0));
    assert_eq!(v1, kv.get(1));
    assert_eq!(2, kv.size());
}

#[test]
#[should_panic(expected = "expected format")]
fn test_parse_illegal_argument() {
    let mut kv = KVectorChain::new();
    kv.parse("{(5,a),(10,50),(30,50)}");
}

#[test]
fn test_get_length() {
    let mut kv = KVectorChain::new();
    kv.parse("{(10,50),(10,50),(10,50)}");
    assert_eq!(0.0, kv.total_length());

    kv.parse("{(10,0),(10,20),(10,30)}");
    assert_eq!(30.0, kv.total_length());
}

#[test]
fn test_get_point_on_line() {
    let v0 = KVector::with_values(5.0, 50.0);
    let v1 = KVector::with_values(10.0, 50.0);
    let v2 = KVector::with_values(30.0, 50.0);
    let kv = KVectorChain::from_vectors(&[v0, v1, v2]);

    assert_eq!(v0, kv.point_on_line(0.0));
    assert_eq!(v1, kv.point_on_line(5.0));
    assert_eq!(v2, kv.point_on_line(40.0));
}

