
use crate::common::elkt_test_loader::{load_layered_graph_from_elk_text, load_layered_graph_from_elkt};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::elk_layered::ElkLayered;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_importer::ElkGraphImporter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    internal_properties::InternalProperties,
    CycleBreakingStrategy, LayeredOptions, LayeringStrategy,
    NodePromotionStrategy, OrderingStrategy,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};

const MIN_EXTERNAL_PTOLEMY_CHECKED: usize = 8;
const MIN_EXTERNAL_PTOLEMY_MODEL_CHECKED: usize = 10;
const MIN_EXTERNAL_MODEL_ORDER_CHECKED: usize = 8;
const MIN_EXTERNAL_NIKOLOV_FAMILY_CHECKED: usize = 8;
const MAX_EXTERNAL_PTOLEMY_SCAN: usize = 120;
const MAX_EXTERNAL_PTOLEMY_MODEL_SCAN: usize = 140;
const MAX_EXTERNAL_MODEL_ORDER_SCAN: usize = 128;
const MAX_EXTERNAL_MODEL_ORDER_CHECKED: usize = 20;
const MAX_EXTERNAL_NIKOLOV_FAMILY_SCAN: usize = 160;
const MAX_EXTERNAL_NIKOLOV_FAMILY_CHECKED: usize = 20;
const MIN_EXTERNAL_PTOLEMY_PARSE_COVERAGE: f64 = 0.35;

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: T,
) {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn build_test_graph() -> (ElkNodeRef, Vec<ElkNodeRef>) {
    let root = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(root.clone()));
    let n2 = ElkGraphUtil::create_node(Some(root.clone()));
    let n3 = ElkGraphUtil::create_node(Some(root.clone()));
    let n4 = ElkGraphUtil::create_node(Some(root.clone()));
    let n5 = ElkGraphUtil::create_node(Some(root.clone()));
    let n6 = ElkGraphUtil::create_node(Some(root.clone()));

    for node in [&n1, &n2, &n3, &n4, &n5, &n6] {
        set_dimensions(node, 30.0, 30.0);
    }

    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n2.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n2.clone()),
        ElkConnectableShapeRef::Node(n5.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n3.clone()),
        ElkConnectableShapeRef::Node(n5.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n4.clone()),
        ElkConnectableShapeRef::Node(n6.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n5.clone()),
        ElkConnectableShapeRef::Node(n6.clone()),
    );

    (root, vec![n1, n2, n3, n4, n5, n6])
}

fn import_lgraph(
    root: &ElkNodeRef,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef {
    let mut origin_store = OriginStore::new();
    let mut importer = ElkGraphImporter::new(&mut origin_store);
    importer.import_graph(root)
}

fn assert_layering_invariants(
    lgraph: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef,
    check_forward_edges: bool,
) {
    let graph_guard = lgraph.lock().expect("lgraph lock");
    let layers = graph_guard.layers().clone();
    if layers.is_empty() {
        return;
    }
    assert!(graph_guard.layerless_nodes().is_empty());
    drop(graph_guard);

    for layer in &layers {
        let layer_guard = layer.lock().expect("layer lock");
        assert!(!layer_guard.nodes().is_empty());
    }

    for layer in &layers {
        let source_layer_index = layer
            .lock()
            .ok()
            .and_then(|layer_guard| layer_guard.index())
            .unwrap_or(0);
        let nodes = layer.lock().expect("layer lock").nodes().clone();
        for node in nodes {
            let outgoing = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing {
                if !check_forward_edges {
                    continue;
                }
                let reversed = edge
                    .lock()
                    .ok()
                    .and_then(|mut edge_guard| {
                        edge_guard.get_property(InternalProperties::REVERSED)
                    })
                    .unwrap_or(false);
                let target_layer_index = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.target())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
                    .and_then(|target_node| {
                        target_node
                            .lock()
                            .ok()
                            .and_then(|node_guard| node_guard.layer())
                    })
                    .and_then(|layer_ref| {
                        layer_ref
                            .lock()
                            .ok()
                            .and_then(|layer_guard| layer_guard.index())
                    })
                    .unwrap_or(source_layer_index);
                if !reversed {
                    assert!(
                        source_layer_index <= target_layer_index,
                        "non-reversed edge should not point to a previous layer"
                    );
                }
            }
        }
    }
}

fn apply_layout_with_promotion(
    root: &ElkNodeRef,
    promotion_strategy: NodePromotionStrategy,
    layering_strategy: LayeringStrategy,
    configure_model_order: bool,
    check_forward_edges: bool,
) {
    set_node_property(
        root,
        CoreOptions::ALGORITHM,
        LayeredOptions::ALGORITHM_ID.to_string(),
    );
    set_node_property(
        root,
        LayeredOptions::LAYERING_NODE_PROMOTION_STRATEGY,
        promotion_strategy,
    );
    set_node_property(root, LayeredOptions::LAYERING_STRATEGY, layering_strategy);

    if configure_model_order {
        set_node_property(
            root,
            LayeredOptions::CYCLE_BREAKING_STRATEGY,
            CycleBreakingStrategy::ModelOrder,
        );
        set_node_property(
            root,
            LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY,
            OrderingStrategy::PreferEdges,
        );
    }

    let lgraph = import_lgraph(root);
    let mut layered = ElkLayered::new();
    layered.do_layout(&lgraph, None);

    assert_layering_invariants(&lgraph, check_forward_edges);
}

fn run_layout_with_promotion(
    promotion_strategy: NodePromotionStrategy,
    layering_strategy: LayeringStrategy,
    configure_model_order: bool,
) {
    initialize_plain_java_layout();
    let (root, _) = build_test_graph();
    apply_layout_with_promotion(
        &root,
        promotion_strategy,
        layering_strategy,
        configure_model_order,
        true,
    );
}

fn load_node_promotion_resource_graph(file_name: &str) -> ElkNodeRef {
    let path = format!(
        "{}/tests/resources/node_promotion/{}",
        env!("CARGO_MANIFEST_DIR"),
        file_name
    );
    load_layered_graph_from_elkt(&path)
        .unwrap_or_else(|err| panic!("node promotion resource should load: {err}"))
}

fn collect_external_ptolemy_resources() -> (Vec<PathBuf>, usize, usize) {
    let mut roots = vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../external/elk/test/org.eclipse.elk.alg.layered.test/src-resources/realworld/ptolemy"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../external/elk/test/org.eclipse.elk.alg.layered.test/src/resources/realworld/ptolemy"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../external/elk-models/realworld/ptolemy"),
    ];

    if let Ok(path) = env::var("ELK_REALWORLD_PTOLEMY_DIR") {
        roots.push(PathBuf::from(path));
    }

    let mut elk_text_files = Vec::new();
    let mut elkt_count = 0usize;
    let mut elkg_count = 0usize;

    for root in roots {
        if !root.exists() {
            continue;
        }
        collect_files_recursively(&root, &mut elk_text_files, &mut elkt_count, &mut elkg_count);
    }

    elk_text_files.sort();
    elk_text_files.dedup();
    (elk_text_files, elkt_count, elkg_count)
}

fn ptolemy_model_key(path: &Path) -> String {
    let parent = path.parent().and_then(|value| value.to_str()).unwrap_or("");
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    format!("{parent}/{stem}")
}

fn collect_external_ptolemy_model_resources() -> (Vec<PathBuf>, usize, usize) {
    let (resources, elkt_count, elkg_count) = collect_external_ptolemy_resources();
    let mut grouped: HashMap<String, (Option<PathBuf>, Option<PathBuf>)> = HashMap::new();

    for resource in resources {
        let key = ptolemy_model_key(&resource);
        let entry = grouped.entry(key).or_insert((None, None));
        match resource.extension().and_then(|ext| ext.to_str()) {
            Some("elkt") => {
                entry.0 = Some(resource);
            }
            Some("elkg") => {
                if entry.1.is_none() {
                    entry.1 = Some(resource);
                }
            }
            _ => {}
        }
    }

    let mut model_resources = grouped
        .into_values()
        .filter_map(|(elkt, elkg)| elkt.or(elkg))
        .collect::<Vec<_>>();
    model_resources.sort();
    (model_resources, elkt_count, elkg_count)
}

fn collect_files_recursively(
    root: &Path,
    elk_text_files: &mut Vec<PathBuf>,
    elkt_count: &mut usize,
    elkg_count: &mut usize,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursively(&path, elk_text_files, elkt_count, elkg_count);
            continue;
        }

        match path.extension().and_then(|ext| ext.to_str()) {
            Some("elkt") => {
                elk_text_files.push(path);
                *elkt_count += 1;
            }
            Some("elkg") => {
                elk_text_files.push(path);
                *elkg_count += 1;
            }
            _ => {}
        }
    }
}

fn sample_resources_spread(resources: &[PathBuf], max_samples: usize) -> Vec<PathBuf> {
    if resources.len() <= max_samples || max_samples <= 1 {
        return resources.to_vec();
    }

    let mut sampled = Vec::with_capacity(max_samples);
    for i in 0..max_samples {
        let index = i * (resources.len() - 1) / (max_samples - 1);
        sampled.push(resources[index].clone());
    }
    sampled
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

#[test]
fn node_promotion_nikolov_preserves_layering_invariants() {
    run_layout_with_promotion(
        NodePromotionStrategy::Nikolov,
        LayeringStrategy::LongestPath,
        false,
    );
}

#[test]
fn node_promotion_nikolov_pixel_preserves_layering_invariants() {
    run_layout_with_promotion(
        NodePromotionStrategy::NikolovPixel,
        LayeringStrategy::LongestPath,
        false,
    );
}

#[test]
fn node_promotion_nikolov_improved_preserves_layering_invariants() {
    run_layout_with_promotion(
        NodePromotionStrategy::NikolovImproved,
        LayeringStrategy::LongestPath,
        false,
    );
}

#[test]
fn node_promotion_nikolov_improved_pixel_preserves_layering_invariants() {
    run_layout_with_promotion(
        NodePromotionStrategy::NikolovImprovedPixel,
        LayeringStrategy::LongestPath,
        false,
    );
}

#[test]
fn node_promotion_no_boundary_preserves_layering_invariants() {
    run_layout_with_promotion(
        NodePromotionStrategy::NoBoundary,
        LayeringStrategy::LongestPath,
        false,
    );
}

#[test]
fn node_promotion_model_order_left_to_right_preserves_layering_invariants() {
    run_layout_with_promotion(
        NodePromotionStrategy::ModelOrderLeftToRight,
        LayeringStrategy::LongestPathSource,
        true,
    );
}

#[test]
fn node_promotion_model_order_right_to_left_preserves_layering_invariants() {
    run_layout_with_promotion(
        NodePromotionStrategy::ModelOrderRightToLeft,
        LayeringStrategy::LongestPath,
        true,
    );
}

#[test]
fn node_promotion_resource_graphs_preserve_layering_invariants_nikolov_family() {
    initialize_plain_java_layout();
    let strategies = [
        NodePromotionStrategy::Nikolov,
        NodePromotionStrategy::NikolovImproved,
        NodePromotionStrategy::NikolovPixel,
        NodePromotionStrategy::NikolovImprovedPixel,
        NodePromotionStrategy::NoBoundary,
    ];
    let resources = ["promotion_case_a.elkt", "promotion_case_b.elkt"];

    for resource in resources {
        for strategy in strategies {
            let root = load_node_promotion_resource_graph(resource);
            apply_layout_with_promotion(
                &root,
                strategy,
                LayeringStrategy::LongestPath,
                false,
                true,
            );
        }
    }
}

#[test]
fn node_promotion_resource_graphs_preserve_layering_invariants_model_order_strategies() {
    initialize_plain_java_layout();
    let resources = ["promotion_case_a.elkt", "promotion_case_b.elkt"];

    for resource in resources {
        let left_to_right = load_node_promotion_resource_graph(resource);
        apply_layout_with_promotion(
            &left_to_right,
            NodePromotionStrategy::ModelOrderLeftToRight,
            LayeringStrategy::LongestPathSource,
            true,
            true,
        );

        let right_to_left = load_node_promotion_resource_graph(resource);
        apply_layout_with_promotion(
            &right_to_left,
            NodePromotionStrategy::ModelOrderRightToLeft,
            LayeringStrategy::LongestPath,
            true,
            true,
        );
    }
}

#[test]
fn node_promotion_external_ptolemy_resources_if_available() {
    initialize_plain_java_layout();
    let (resources, elkt_count, elkg_count) = collect_external_ptolemy_resources();

    if resources.is_empty() {
        eprintln!(
            "node_promotion: no external ptolemy .elkt/.elkg resources found (detected elkt={elkt_count}, elkg={elkg_count})"
        );
        return;
    }

    let sampled_resources = sample_resources_spread(&resources, MAX_EXTERNAL_PTOLEMY_SCAN);
    let mut checked = 0usize;
    let mut parse_failures = Vec::new();
    let mut layout_failures = Vec::new();
    for resource in &sampled_resources {
        let path = resource.to_string_lossy().into_owned();
        let root = match load_layered_graph_from_elk_text(&path) {
            Ok(root) => root,
            Err(err) => {
                parse_failures.push(format!("{path}: {err}"));
                continue;
            }
        };
        if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(|| {
            apply_layout_with_promotion(
                &root,
                NodePromotionStrategy::Nikolov,
                LayeringStrategy::LongestPath,
                false,
                false,
            )
        })) {
            layout_failures.push(format!("{path}: {}", panic_payload_to_string(payload)));
            continue;
        }
        checked += 1;
    }

    if checked < MIN_EXTERNAL_PTOLEMY_CHECKED {
        let sample = parse_failures
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ");
        panic!(
            "node_promotion: expected at least {MIN_EXTERNAL_PTOLEMY_CHECKED} validated external resources, got checked={checked} (sampled={}, parse_failures={}, layout_failures={}, sample failures={})",
            sampled_resources.len(),
            parse_failures.len(),
            layout_failures.len(),
            sample
        );
    } else if !parse_failures.is_empty() || !layout_failures.is_empty() {
        eprintln!(
            "node_promotion: checked {checked} external ptolemy resources (sampled={}), parse failures={} (sample: {}), layout_failures={} (sample: {})",
            sampled_resources.len(),
            parse_failures.len(),
            parse_failures
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(" | "),
            layout_failures.len(),
            layout_failures
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
}

#[test]
fn node_promotion_external_ptolemy_model_parse_coverage_if_available() {
    initialize_plain_java_layout();
    let (resources, elkt_count, elkg_count) = collect_external_ptolemy_model_resources();

    if resources.is_empty() {
        eprintln!(
            "node_promotion(coverage): no external ptolemy model resources found (detected elkt={elkt_count}, elkg={elkg_count})"
        );
        eprintln!("METRIC:ptolemy_parse_coverage parsed=0 sampled=0 coverage=0.000");
        return;
    }

    let sampled_resources = sample_resources_spread(&resources, MAX_EXTERNAL_PTOLEMY_MODEL_SCAN);
    let parsed = sampled_resources
        .iter()
        .filter(|resource| load_layered_graph_from_elk_text(&resource.to_string_lossy()).is_ok())
        .count();
    let parse_coverage = parsed as f64 / sampled_resources.len() as f64;
    eprintln!(
        "METRIC:ptolemy_parse_coverage parsed={parsed} sampled={} coverage={parse_coverage:.3}",
        sampled_resources.len()
    );

    assert!(
        parsed >= MIN_EXTERNAL_PTOLEMY_MODEL_CHECKED,
        "node_promotion(coverage): expected at least {MIN_EXTERNAL_PTOLEMY_MODEL_CHECKED} parsed model resources, got parsed={parsed}, sampled={}, parse_coverage={parse_coverage:.3}",
        sampled_resources.len()
    );
    assert!(
        parse_coverage >= MIN_EXTERNAL_PTOLEMY_PARSE_COVERAGE,
        "node_promotion(coverage): expected parse coverage >= {MIN_EXTERNAL_PTOLEMY_PARSE_COVERAGE:.3}, got {parse_coverage:.3} (parsed={parsed}, sampled={})",
        sampled_resources.len()
    );
}

#[test]
fn node_promotion_external_ptolemy_resources_model_order_if_available() {
    initialize_plain_java_layout();
    let (resources, elkt_count, elkg_count) = collect_external_ptolemy_model_resources();

    if resources.is_empty() {
        eprintln!(
            "node_promotion(model-order): no external ptolemy model resources found (detected elkt={elkt_count}, elkg={elkg_count})"
        );
        eprintln!("METRIC:ptolemy_model_order_validated checked=0 sampled=0");
        return;
    }

    let sampled_resources = sample_resources_spread(&resources, MAX_EXTERNAL_MODEL_ORDER_SCAN);
    let mut checked = 0usize;
    let mut parse_failures = Vec::new();
    let mut layout_failures = Vec::new();
    for resource in &sampled_resources {
        let path = resource.to_string_lossy().into_owned();
        let left_to_right = match load_layered_graph_from_elk_text(&path) {
            Ok(root) => root,
            Err(err) => {
                parse_failures.push(format!("{path}: {err}"));
                continue;
            }
        };
        let left_to_right_result = panic::catch_unwind(AssertUnwindSafe(|| {
            apply_layout_with_promotion(
                &left_to_right,
                NodePromotionStrategy::ModelOrderLeftToRight,
                LayeringStrategy::LongestPathSource,
                true,
                false,
            )
        }));
        if let Err(payload) = left_to_right_result {
            layout_failures.push(format!("{path}: {}", panic_payload_to_string(payload)));
            continue;
        }

        let right_to_left = match load_layered_graph_from_elk_text(&path) {
            Ok(root) => root,
            Err(err) => {
                parse_failures.push(format!("{path}: {err}"));
                continue;
            }
        };
        let right_to_left_result = panic::catch_unwind(AssertUnwindSafe(|| {
            apply_layout_with_promotion(
                &right_to_left,
                NodePromotionStrategy::ModelOrderRightToLeft,
                LayeringStrategy::LongestPath,
                true,
                false,
            )
        }));
        if let Err(payload) = right_to_left_result {
            layout_failures.push(format!("{path}: {}", panic_payload_to_string(payload)));
            continue;
        }

        checked += 1;
        if checked >= MAX_EXTERNAL_MODEL_ORDER_CHECKED {
            break;
        }
    }

    eprintln!(
        "METRIC:ptolemy_model_order_validated checked={checked} sampled={}",
        sampled_resources.len()
    );

    if checked < MIN_EXTERNAL_MODEL_ORDER_CHECKED {
        let sample = parse_failures
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ");
        panic!(
            "node_promotion(model-order): expected at least {MIN_EXTERNAL_MODEL_ORDER_CHECKED} validated external resources, got checked={checked} (sampled={}, parse_failures={}, layout_failures={}, sample failures={})",
            sampled_resources.len(),
            parse_failures.len(),
            layout_failures.len(),
            sample
        );
    } else if !parse_failures.is_empty() || !layout_failures.is_empty() {
        eprintln!(
            "node_promotion(model-order): checked {checked} external resources (sampled={}), parse failures={} (sample: {}), layout_failures={} (sample: {})",
            sampled_resources.len(),
            parse_failures.len(),
            parse_failures
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(" | "),
            layout_failures.len(),
            layout_failures
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
}

#[test]
fn node_promotion_external_ptolemy_resources_nikolov_family_if_available() {
    initialize_plain_java_layout();
    let (resources, elkt_count, elkg_count) = collect_external_ptolemy_model_resources();

    if resources.is_empty() {
        eprintln!(
            "node_promotion(nikolov-family): no external ptolemy model resources found (detected elkt={elkt_count}, elkg={elkg_count})"
        );
        return;
    }

    let sampled_resources = sample_resources_spread(&resources, MAX_EXTERNAL_NIKOLOV_FAMILY_SCAN);
    let mut checked = 0usize;
    let mut parse_failures = Vec::new();
    let mut layout_failures = Vec::new();
    for resource in &sampled_resources {
        let path = resource.to_string_lossy().into_owned();
        let nikolov = match load_layered_graph_from_elk_text(&path) {
            Ok(root) => root,
            Err(err) => {
                parse_failures.push(format!("{path}: {err}"));
                continue;
            }
        };
        let nikolov_result = panic::catch_unwind(AssertUnwindSafe(|| {
            apply_layout_with_promotion(
                &nikolov,
                NodePromotionStrategy::Nikolov,
                LayeringStrategy::LongestPath,
                false,
                false,
            )
        }));
        if let Err(payload) = nikolov_result {
            layout_failures.push(format!("{path}: {}", panic_payload_to_string(payload)));
            continue;
        }

        let nikolov_pixel = match load_layered_graph_from_elk_text(&path) {
            Ok(root) => root,
            Err(err) => {
                parse_failures.push(format!("{path}: {err}"));
                continue;
            }
        };
        let nikolov_pixel_result = panic::catch_unwind(AssertUnwindSafe(|| {
            apply_layout_with_promotion(
                &nikolov_pixel,
                NodePromotionStrategy::NikolovPixel,
                LayeringStrategy::LongestPath,
                false,
                false,
            )
        }));
        if let Err(payload) = nikolov_pixel_result {
            layout_failures.push(format!("{path}: {}", panic_payload_to_string(payload)));
            continue;
        }

        let nikolov_improved = match load_layered_graph_from_elk_text(&path) {
            Ok(root) => root,
            Err(err) => {
                parse_failures.push(format!("{path}: {err}"));
                continue;
            }
        };
        let nikolov_improved_result = panic::catch_unwind(AssertUnwindSafe(|| {
            apply_layout_with_promotion(
                &nikolov_improved,
                NodePromotionStrategy::NikolovImproved,
                LayeringStrategy::LongestPath,
                false,
                false,
            )
        }));
        if let Err(payload) = nikolov_improved_result {
            layout_failures.push(format!("{path}: {}", panic_payload_to_string(payload)));
            continue;
        }

        let nikolov_improved_pixel = match load_layered_graph_from_elk_text(&path) {
            Ok(root) => root,
            Err(err) => {
                parse_failures.push(format!("{path}: {err}"));
                continue;
            }
        };
        let nikolov_improved_pixel_result = panic::catch_unwind(AssertUnwindSafe(|| {
            apply_layout_with_promotion(
                &nikolov_improved_pixel,
                NodePromotionStrategy::NikolovImprovedPixel,
                LayeringStrategy::LongestPath,
                false,
                false,
            )
        }));
        if let Err(payload) = nikolov_improved_pixel_result {
            layout_failures.push(format!("{path}: {}", panic_payload_to_string(payload)));
            continue;
        }

        let no_boundary = match load_layered_graph_from_elk_text(&path) {
            Ok(root) => root,
            Err(err) => {
                parse_failures.push(format!("{path}: {err}"));
                continue;
            }
        };
        let no_boundary_result = panic::catch_unwind(AssertUnwindSafe(|| {
            apply_layout_with_promotion(
                &no_boundary,
                NodePromotionStrategy::NoBoundary,
                LayeringStrategy::LongestPath,
                false,
                false,
            )
        }));
        if let Err(payload) = no_boundary_result {
            layout_failures.push(format!("{path}: {}", panic_payload_to_string(payload)));
            continue;
        }

        checked += 1;
        if checked >= MAX_EXTERNAL_NIKOLOV_FAMILY_CHECKED {
            break;
        }
    }

    if checked < MIN_EXTERNAL_NIKOLOV_FAMILY_CHECKED {
        let sample = parse_failures
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ");
        panic!(
            "node_promotion(nikolov-family): expected at least {MIN_EXTERNAL_NIKOLOV_FAMILY_CHECKED} validated external resources, got checked={checked} (sampled={}, parse_failures={}, layout_failures={}, sample failures={})",
            sampled_resources.len(),
            parse_failures.len(),
            layout_failures.len(),
            sample
        );
    } else if !parse_failures.is_empty() || !layout_failures.is_empty() {
        eprintln!(
            "node_promotion(nikolov-family): checked {checked} external resources (sampled={}), parse failures={} (sample: {}), layout_failures={} (sample: {})",
            sampled_resources.len(),
            parse_failures.len(),
            parse_failures
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(" | "),
            layout_failures.len(),
            layout_failures
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
}
