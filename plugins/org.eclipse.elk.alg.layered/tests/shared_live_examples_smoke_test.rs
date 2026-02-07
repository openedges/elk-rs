mod elkt_test_loader;
mod issue_support;

use std::path::{Path, PathBuf};

use elkt_test_loader::load_graph_from_elkt;
use issue_support::{init_layered_options, run_recursive_layout};

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const LIVE_EXAMPLES: &[&str] = &[
    "external/elk/plugins/org.eclipse.elk.alg.layered/images/example.elkt",
    "external/elk/plugins/org.eclipse.elk.core/images/exampleBox.elkt",
    "external/elk/plugins/org.eclipse.elk.core/images/exampleRandomizer.elkt",
];

#[test]
fn live_examples_load_and_layout_without_exceptions() {
    init_layered_options();

    for relative_path in LIVE_EXAMPLES {
        let path = workspace_path(relative_path);
        assert!(
            path.exists(),
            "example file should exist: {}",
            path.display()
        );
        let path_str = path.to_string_lossy().to_string();

        let graph =
            load_graph_from_elkt(path_str.as_str(), Some(LayeredOptions::ALGORITHM_ID))
                .unwrap_or_else(|err| {
                    panic!("failed to load example '{}': {err}", path.display());
                });

        run_recursive_layout(&graph);
        assert_graph_has_finite_node_geometry(path_str.as_str(), &graph);
    }
}

fn workspace_path(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative_path)
}

fn assert_graph_has_finite_node_geometry(path: &str, root: &ElkNodeRef) {
    let mut nodes = vec![root.clone()];
    let mut index = 0usize;
    while index < nodes.len() {
        let current = nodes[index].clone();
        let mut current_guard = current.borrow_mut();
        let shape = current_guard.connectable().shape();
        assert!(
            shape.x().is_finite() && shape.y().is_finite(),
            "node coordinates should be finite for example '{path}'"
        );
        assert!(
            shape.width().is_finite() && shape.height().is_finite(),
            "node size should be finite for example '{path}'"
        );

        let children: Vec<ElkNodeRef> = current_guard.children().iter().cloned().collect();
        nodes.extend(children);
        index += 1;
    }

    assert!(
        nodes.len() > 1,
        "example '{path}' should contain at least one child node"
    );
}
