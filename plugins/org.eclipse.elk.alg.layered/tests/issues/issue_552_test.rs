
use crate::common::elkt_test_loader::{find_node_by_identifier, find_port_by_identifier, load_layered_graph_from_elkt};
use crate::common::issue_support::{init_layered_options, run_layout, run_recursive_layout};

#[test]
fn issue_552_self_loop_ports_are_placed_away_from_zero_y() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_552_self_loop_ports.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_552 resource should load");

    run_layout(&graph);

    for identifier in ["p_west", "p_east"] {
        let port =
            find_port_by_identifier(&graph, identifier).expect("expected self-loop port to exist");
        let mut port_mut = port.borrow_mut();
        let shape = port_mut.connectable().shape();
        let y = shape.y();
        assert!(y > 0.0, "port y should be > 0.0, got {y}");
    }
}

#[test]
fn issue_552_hierarchical_self_loop_source_port_matches_java_y() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_552_hierarchy_self_loop_ports.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_552 hierarchy resource should load");

    run_recursive_layout(&graph);

    let src_port =
        find_port_by_identifier(&graph, "p_loop_src").expect("expected source self-loop port");
    let target_port =
        find_port_by_identifier(&graph, "p_loop_tgt").expect("expected target self-loop port");
    let child_node = find_node_by_identifier(&graph, "n2").expect("expected child node n2");

    let src_y = src_port.borrow_mut().connectable().shape().y();
    let target_y = target_port.borrow_mut().connectable().shape().y();
    let child_y = child_node.borrow_mut().connectable().shape().y();

    assert!(
        (src_y - 12.0).abs() <= 1e-6,
        "source port y should match Java (12.0), got {src_y}"
    );
    assert!(
        (target_y - 32.0).abs() <= 1e-6,
        "target port y should match Java (32.0), got {target_y}"
    );
    assert!(
        (child_y - 12.0).abs() <= 1e-6,
        "child node y should match Java (12.0), got {child_y}"
    );
}
