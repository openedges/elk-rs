use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use crate::common::elkt_test_loader::load_graph_from_elkt;

use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::options::{
    ForceMetaDataProvider, StressMetaDataProvider, StressOptions,
};
use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::stress::StressLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::options::{
    RectPackingMetaDataProvider, RectPackingOptions,
};
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::RectPackingLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    AlgorithmFactory, InstancePool, NullElkProgressMonitor,
};
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

#[test]
fn elk_live_examples_test() {
    init_live_examples_layout();

    let examples_root = workspace_path("external/elk-models/examples");
    assert!(
        examples_root.exists(),
        "examples directory should exist: {}",
        examples_root.display()
    );

    let mut example_files = Vec::new();
    collect_elkt_files(&examples_root, &mut example_files);
    example_files.sort();

    assert!(
        !example_files.is_empty(),
        "expected to find .elkt examples under {}",
        examples_root.display()
    );

    let mut failures = Vec::new();
    for path in &example_files {
        let path_str = path.to_string_lossy().to_string();
        let graph = load_graph_from_elkt(path_str.as_str(), None).unwrap_or_else(|err| {
            panic!("failed to load example '{}': {err}", path.display());
        });

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_recursive_layout(&graph);
            assert_graph_has_finite_node_geometry(path_str.as_str(), &graph);
        }));
        if let Err(e) = result {
            let msg = e
                .downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap_or("unknown panic");
            failures.push(format!("{}: {}", path.display(), msg));
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} of {} examples failed:\n{}",
            failures.len(),
            example_files.len(),
            failures.join("\n")
        );
    }
}

fn init_live_examples_layout() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        initialize_plain_java_layout();

        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&RectPackingMetaDataProvider);
        service.register_layout_meta_data_provider(&ForceMetaDataProvider);
        service.register_layout_meta_data_provider(&StressMetaDataProvider);

        let rect_factory = AlgorithmFactory::new(|| Box::new(RectPackingLayoutProvider::new()));
        let rect_pool = InstancePool::new(Box::new(rect_factory));
        service.override_algorithm_provider_pool(
            RectPackingOptions::ALGORITHM_ID,
            Arc::new(rect_pool),
        );

        let stress_factory = AlgorithmFactory::new(|| Box::new(StressLayoutProvider::new()));
        let stress_pool = InstancePool::new(Box::new(stress_factory));
        service
            .override_algorithm_provider_pool(StressOptions::ALGORITHM_ID, Arc::new(stress_pool));
    });
}

fn run_recursive_layout(graph: &ElkNodeRef) {
    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = NullElkProgressMonitor;
    engine.layout(graph, &mut monitor);
}

fn collect_elkt_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("failed to read directory '{}': {err}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|err| {
            panic!(
                "failed to read directory entry in '{}': {err}",
                dir.display()
            )
        });
        let path = entry.path();
        if path.is_dir() {
            collect_elkt_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("elkt") {
            files.push(path);
        }
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
