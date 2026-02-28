
use std::path::PathBuf;

use crate::common::elkt_test_loader::{find_edge_by_identifier, load_layered_graph_from_elkt};
use crate::common::issue_support::{init_layered_options, run_recursive_layout};
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeRef;

const EPSILON: f64 = 1e-6;

#[test]
fn polyline_self_loop_labels_keeps_java_corner_cut_duplicates() {
    init_layered_options();

    let resource = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../external/elk-models/tickets/layered/079_selfLoopLabels.elkt");
    if !resource.exists() {
        eprintln!(
            "polyline self-loop resource missing, skipping: {}",
            resource.display()
        );
        return;
    }

    let path = resource.to_string_lossy();
    let graph =
        load_layered_graph_from_elkt(path.as_ref()).expect("079_selfLoopLabels should load");
    run_recursive_layout(&graph);

    let edge = find_edge_by_identifier(&graph, "n1", "n1").expect("self-loop edge should exist");
    let bends = edge_bend_points(&edge);
    assert_eq!(
        bends.len(),
        4,
        "polyline self-loop should keep Java-equivalent 4 bend points"
    );
    assert!(
        bends
            .windows(2)
            .any(|pair| (pair[0].x - pair[1].x).abs() < EPSILON
                && (pair[0].y - pair[1].y).abs() < EPSILON),
        "polyline corner cutting should preserve at least one duplicate pair: {bends:?}"
    );
}

#[test]
fn polyline_self_loop_issue_463_keeps_java_corner_cut_path_density() {
    init_layered_options();

    let resource = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../external/elk-models/tickets/layered/463_aioobe_with_self_loops.elkt");
    if !resource.exists() {
        eprintln!(
            "polyline self-loop resource missing, skipping: {}",
            resource.display()
        );
        return;
    }

    let path = resource.to_string_lossy();
    let graph = load_layered_graph_from_elkt(path.as_ref())
        .expect("463_aioobe_with_self_loops should load");
    run_recursive_layout(&graph);

    let edge = find_edge_by_identifier(&graph, "requiredBundles", "usedByBundles")
        .expect("E2 edge should exist");
    let bends = edge_bend_points(&edge);
    assert_eq!(
        bends.len(),
        8,
        "polyline self-loop should match Java 8-point corner-cut path"
    );
}

fn edge_bend_points(edge: &ElkEdgeRef) -> Vec<KVector> {
    let section = {
        let mut edge_mut = edge.borrow_mut();
        edge_mut
            .sections()
            .get(0)
            .expect("edge should have at least one section")
    };

    let mut section_mut = section.borrow_mut();
    section_mut
        .bend_points()
        .iter()
        .map(|bend_ref| {
            let bend = bend_ref.borrow();
            KVector::with_values(bend.x(), bend.y())
        })
        .collect()
}
