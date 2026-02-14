mod elkt_test_loader;
mod issue_support;

use std::path::PathBuf;

use elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_recursive_layout};
use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;
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

#[test]
fn ordering_realworld_ci_router_drop_queue_matches_java() {
    init_layered_options();

    let resource = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../external/elk-models/realworld/ptolemy/flattened/ci_router_dropqueuetest1.elkt",
    );
    if !resource.exists() {
        eprintln!(
            "ordering realworld resource missing, skipping: {}",
            resource.display()
        );
        return;
    }

    let path = resource.to_string_lossy();
    let graph =
        load_layered_graph_from_elkt(path.as_ref()).expect("ci_router_dropqueuetest1 should load");
    run_recursive_layout(&graph);

    let n15 = find_node_by_identifier(&graph, "N15").expect("N15 should exist");
    let n1 = find_node_by_identifier(&graph, "N1").expect("N1 should exist");
    let n6 = find_node_by_identifier(&graph, "N6").expect("N6 should exist");

    let n15_y = node_y(&n15);
    let n1_y = node_y(&n1);
    let n6_y = node_y(&n6);
    assert!(
        n15_y + EPSILON < n1_y && n1_y + EPSILON < n6_y,
        "expected Java order N15 above N1 above N6 (n15_y={n15_y}, n1_y={n1_y}, n6_y={n6_y})"
    );
}

#[test]
fn ordering_realworld_random_sequence_matches_java_reference() {
    let mut random = Random::new(1);
    assert_eq!(random.next_int(1), 0);
    assert_eq!(random.next_int(1), 0);

    let seed = random.next_long();
    assert_eq!(seed, 7564655870752979346);

    let distributor_node_relative = random.next_boolean();
    assert!(
        !distributor_node_relative,
        "expected LayerTotal port distributor for Java parity"
    );

    random.set_seed(seed as u64);
    let first_sweep_forward = random.next_boolean();
    assert!(
        first_sweep_forward,
        "expected first randomized sweep to start forward for Java parity"
    );
}

#[test]
fn ordering_realworld_algebraic_heater_open_tank_matches_java() {
    init_layered_options();

    let resource = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../external/elk-models/realworld/ptolemy/flattened/algebraic_heateropentank_HeaterOpenTankRefactored.elkt",
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
        .expect("algebraic_heateropentank_HeaterOpenTankRefactored should load");
    run_recursive_layout(&graph);

    let n1 = find_node_by_identifier(&graph, "N1").expect("N1 should exist");
    let n10 = find_node_by_identifier(&graph, "N10").expect("N10 should exist");
    let n8 = find_node_by_identifier(&graph, "N8").expect("N8 should exist");
    assert!(
        node_y(&n1) + EPSILON < node_y(&n10) && node_y(&n10) + EPSILON < node_y(&n8),
        "expected Java order N1 above N10 above N8"
    );

    let n13 = find_node_by_identifier(&graph, "N13").expect("N13 should exist");
    let n4 = find_node_by_identifier(&graph, "N4").expect("N4 should exist");
    assert!(
        node_y(&n13) + EPSILON < node_y(&n4),
        "expected Java order N13 above N4"
    );

    let n5 = find_node_by_identifier(&graph, "N5").expect("N5 should exist");
    let n6 = find_node_by_identifier(&graph, "N6").expect("N6 should exist");
    assert!(
        node_y(&n5) + EPSILON < node_y(&n6),
        "expected Java order N5 above N6"
    );
}

fn node_y(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().y()
}
