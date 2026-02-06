mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::load_layered_graph_from_elkt;
use issue_support::{init_layered_options, run_layout};

#[test]
fn issue_587_layout_runs_for_all_supported_edge_routings() {
    init_layered_options();

    let resource_paths = [
        format!(
            "{}/tests/resources/issues/issue_587_orthogonal.elkt",
            env!("CARGO_MANIFEST_DIR")
        ),
        format!(
            "{}/tests/resources/issues/issue_587_polyline.elkt",
            env!("CARGO_MANIFEST_DIR")
        ),
        format!(
            "{}/tests/resources/issues/issue_587_splines.elkt",
            env!("CARGO_MANIFEST_DIR")
        ),
    ];

    for path in resource_paths {
        let graph = load_layered_graph_from_elkt(&path).expect("issue_587 resource should load");
        run_layout(&graph);

        let edges = graph
            .borrow_mut()
            .contained_edges()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(edges.len(), 2, "expected two edges in {path}");

        for edge in edges {
            assert!(!edge.borrow_mut().sections().is_empty());
        }
    }
}
