mod elkt_test_loader;
mod issue_support;

use std::path::PathBuf;

use elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_recursive_layout};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const EPSILON: f64 = 0.1;

#[test]
fn ordering_realworld_check_execution_time_constraints_matches_java() {
    init_layered_options();

    let resource = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../external/elk-models/realworld/ptolemy/flattened/aspect_compositeqm_CheckExecutionTimeConstraints.elkt",
    );
    if !resource.exists() {
        eprintln!(
            "ordering realworld resource missing, skipping: {}",
            resource.display()
        );
        return;
    }

    let path = resource.to_string_lossy();
    let graph = load_layered_graph_from_elkt(path.as_ref())
        .expect("aspect_compositeqm_CheckExecutionTimeConstraints should load");
    run_recursive_layout(&graph);

    let n9 = find_node_by_identifier(&graph, "N9").expect("N9 should exist");
    let n10 = find_node_by_identifier(&graph, "N10").expect("N10 should exist");

    let n9_y = node_y(&n9);
    let n10_y = node_y(&n10);
    assert!(
        n9_y + EPSILON < n10_y,
        "expected Java order N9 above N10 (n9_y={n9_y}, n10_y={n10_y})"
    );
}

fn node_y(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().y()
}
