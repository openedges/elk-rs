mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_port_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_layout};

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
