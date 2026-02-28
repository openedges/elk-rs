use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::counting::BinaryIndexedTree;

#[test]
fn sum_before() {
    let mut ft = BinaryIndexedTree::new(5);
    ft.add(1);
    ft.add(2);
    ft.add(1);

    assert_eq!(ft.rank(1), 0);
    assert_eq!(ft.rank(2), 2);
}

#[test]
fn size() {
    let mut ft = BinaryIndexedTree::new(5);
    ft.add(2);
    ft.add(1);
    ft.add(1);

    assert_eq!(ft.size(), 3);
}

#[test]
fn remove_all() {
    let mut ft = BinaryIndexedTree::new(5);
    ft.add(0);
    ft.add(2);
    ft.add(1);
    ft.add(1);

    ft.remove_all(1);

    assert_eq!(ft.size(), 2);
    assert_eq!(ft.rank(2), 1);

    ft.remove_all(1);

    assert_eq!(ft.size(), 2);
    assert_eq!(ft.rank(2), 1);
}

#[test]
fn rank_with_out_of_bounds_index_returns_prefix_sum_up_to_max() {
    let mut ft = BinaryIndexedTree::new(2);
    ft.add(0);
    ft.add(1);
    ft.add(1);

    assert_eq!(ft.rank(usize::MAX), 3);
}
