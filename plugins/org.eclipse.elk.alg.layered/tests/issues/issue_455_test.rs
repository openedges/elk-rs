
use crate::common::elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use crate::common::issue_support::{init_layered_options, run_recursive_layout};

const COORDINATE_FUZZINESS: f64 = 0.5;

#[test]
fn issue_455_compound_children_share_layer_when_targeting_same_node() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_455_compound_children_same_layer.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_455 resource should load");

    run_recursive_layout(&graph);

    let c1 = find_node_by_identifier(&graph, "c1").expect("c1 node should exist");
    let c2 = find_node_by_identifier(&graph, "c2").expect("c2 node should exist");

    let c1_x = c1.borrow_mut().connectable().shape().x();
    let c2_x = c2.borrow_mut().connectable().shape().x();
    assert!(
        (c1_x - c2_x).abs() <= COORDINATE_FUZZINESS,
        "children not in same layer: c1_x={c1_x}, c2_x={c2_x}"
    );
}
