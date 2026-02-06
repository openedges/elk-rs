mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_edge_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_layout};

const COORDINATE_FUZZINESS: f64 = 0.5;

#[test]
fn issue_471_multiple_edge_labels_are_horizontally_centered() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_471_multiple_edge_labels.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_471 resource should load");

    run_layout(&graph);

    let edge =
        find_edge_by_identifier(&graph, "source", "target").expect("main edge source->target should exist");
    let labels = edge
        .borrow_mut()
        .element()
        .labels()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(labels.len(), 3, "expected three edge labels");

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;

    for label in labels {
        let mut label_mut = label.borrow_mut();
        let shape = label_mut.shape();
        min_x = min_x.min(shape.x());
        max_x = max_x.max(shape.x());
        assert!(
            shape.x().is_finite() && shape.y().is_finite() && shape.width() > 0.0 && shape.height() > 0.0,
            "label geometry is invalid: ({}, {}, {}, {})",
            shape.x(),
            shape.y(),
            shape.width(),
            shape.height()
        );
    }

    assert!(
        (max_x - min_x).abs() <= COORDINATE_FUZZINESS,
        "edge labels are not horizontally aligned: min_x={min_x}, max_x={max_x}"
    );
}
