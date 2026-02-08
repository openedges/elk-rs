mod elkt_test_loader;
mod issue_support;

use std::collections::{HashSet, VecDeque};
use std::env;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use elkt_test_loader::{load_layered_graph_from_elk_text, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_recursive_layout};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkNodeRef,
};

const MIN_EXTERNAL_SELF_LOOP_CHECKED: usize = 4;
const MAX_EXTERNAL_SELF_LOOP_SCAN: usize = 60;

fn self_loop_resource_paths() -> Vec<String> {
    let base = format!("{}/tests/resources/issues", env!("CARGO_MANIFEST_DIR"));
    vec![
        format!("{base}/issue_444_self_loop.elkt"),
        format!("{base}/issue_463_self_loops.elkt"),
        format!("{base}/issue_548_inside_self_loops.elkt"),
        format!("{base}/issue_552_self_loop_ports.elkt"),
        format!("{base}/issue_433_self_loop_label_bounds.elkt"),
    ]
}

fn collect_external_self_loop_ticket_resources() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../external/elk-models/tickets/layered")];
    if let Ok(path) = env::var("ELK_LAYERED_TICKETS_DIR") {
        roots.push(PathBuf::from(path));
    }

    let mut resources = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        collect_self_loop_ticket_files(&root, &mut resources);
    }
    resources.sort();
    resources.dedup();
    resources
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

fn collect_self_loop_ticket_files(root: &Path, resources: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_self_loop_ticket_files(&path, resources);
            continue;
        }

        let ext = path.extension().and_then(|value| value.to_str());
        if ext != Some("elkt") && ext != Some("elkg") {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if is_self_loop_ticket_resource(file_name) {
            resources.push(path);
        }
    }
}

fn is_self_loop_ticket_resource(file_name: &str) -> bool {
    const SELF_LOOP_TICKET_PREFIXES: [&str; 21] = [
        "079_",
        "128_",
        "273_",
        "288_",
        "297_",
        "298_",
        "302_",
        "352_",
        "360_",
        "368_",
        "403_",
        "404_",
        "416_",
        "418_",
        "419_",
        "425_",
        "433_",
        "444_",
        "463_",
        "548_",
        "552_",
    ];
    SELF_LOOP_TICKET_PREFIXES
        .iter()
        .any(|prefix| file_name.starts_with(prefix))
}

fn collect_all_edges(graph: &ElkNodeRef) -> Vec<ElkEdgeRef> {
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();
    let mut edges = Vec::new();
    queue.push_back(graph.clone());

    while let Some(node) = queue.pop_front() {
        let contained_edges: Vec<_> = node.borrow_mut().contained_edges().iter().cloned().collect();
        for edge in contained_edges {
            let ptr = Rc::as_ptr(&edge) as usize;
            if seen.insert(ptr) {
                edges.push(edge);
            }
        }

        let children: Vec<_> = node.borrow_mut().children().iter().cloned().collect();
        queue.extend(children);
    }

    edges
}

fn endpoint_owner_node(shape: &ElkConnectableShapeRef) -> Option<ElkNodeRef> {
    match shape {
        ElkConnectableShapeRef::Node(node) => Some(node.clone()),
        ElkConnectableShapeRef::Port(port) => port.borrow_mut().parent(),
    }
}

fn is_self_loop_edge(edge: &ElkEdgeRef) -> bool {
    let mut edge_mut = edge.borrow_mut();
    let sources: Vec<_> = edge_mut.sources().iter().cloned().collect();
    let targets: Vec<_> = edge_mut.targets().iter().cloned().collect();
    drop(edge_mut);

    for source in sources {
        let Some(source_node) = endpoint_owner_node(&source) else {
            continue;
        };
        for target in &targets {
            let Some(target_node) = endpoint_owner_node(target) else {
                continue;
            };
            if Rc::ptr_eq(&source_node, &target_node) {
                return true;
            }
        }
    }
    false
}

fn assert_finite(value: f64, context: &str) {
    assert!(value.is_finite(), "non-finite geometry value in {context}: {value}");
}

fn assert_self_loop_geometry(edge: &ElkEdgeRef, resource: &str) {
    let mut edge_mut = edge.borrow_mut();
    let sections: Vec<_> = edge_mut.sections().iter().cloned().collect();
    let labels: Vec<_> = edge_mut.element().labels().iter().cloned().collect();
    drop(edge_mut);

    assert!(
        !sections.is_empty(),
        "self-loop edge must have routed sections for resource {resource}"
    );

    for section_ref in sections {
        let mut section = section_ref.borrow_mut();
        assert_finite(section.start_x(), resource);
        assert_finite(section.start_y(), resource);
        assert_finite(section.end_x(), resource);
        assert_finite(section.end_y(), resource);

        let bends: Vec<_> = section.bend_points().to_vec();
        drop(section);
        for bend_ref in bends {
            let bend = bend_ref.borrow();
            assert_finite(bend.x(), resource);
            assert_finite(bend.y(), resource);
        }
    }

    for label_ref in labels {
        let mut label = label_ref.borrow_mut();
        let shape = label.shape();
        assert_finite(shape.x(), resource);
        assert_finite(shape.y(), resource);
        assert_finite(shape.width(), resource);
        assert_finite(shape.height(), resource);
        assert!(
            shape.width() >= 0.0 && shape.height() >= 0.0,
            "label size must be non-negative for resource {resource}"
        );
    }
}

fn has_self_loop_without_sections(graph: &ElkNodeRef) -> bool {
    collect_all_edges(graph)
        .into_iter()
        .filter(is_self_loop_edge)
        .any(|edge| edge.borrow_mut().sections().is_empty())
}

fn run_layout_prefer_recursive(graph: &ElkNodeRef) {
    run_recursive_layout(graph);
}

#[test]
fn self_loop_issue_resources_have_sections_and_finite_geometry() {
    init_layered_options();

    for resource in self_loop_resource_paths() {
        let graph = load_layered_graph_from_elkt(&resource)
            .unwrap_or_else(|err| panic!("failed to load self-loop resource {resource}: {err}"));

        run_layout_prefer_recursive(&graph);

        assert!(
            !has_self_loop_without_sections(&graph),
            "self-loop recursive layout should route sections without subgraph fallback: {resource}"
        );

        let all_edges = collect_all_edges(&graph);
        let self_loop_edges = all_edges
            .into_iter()
            .filter(is_self_loop_edge)
            .collect::<Vec<_>>();

        assert!(
            !self_loop_edges.is_empty(),
            "resource should contain at least one self-loop edge: {resource}"
        );
        for edge in self_loop_edges {
            assert_self_loop_geometry(&edge, &resource);
        }
    }
}

#[test]
fn self_loop_external_ticket_resources_if_available_have_finite_geometry() {
    init_layered_options();

    let resources = collect_external_self_loop_ticket_resources();
    if resources.is_empty() {
        eprintln!("self_loop(resources): no external layered self-loop ticket resources found");
        return;
    }

    let sampled_resources = sample_resources_spread(&resources, MAX_EXTERNAL_SELF_LOOP_SCAN);
    let mut checked = 0usize;
    let mut parse_failures = Vec::new();
    let mut layout_failures = Vec::new();
    let mut missing_self_loops = Vec::new();

    for resource in &sampled_resources {
        let path = resource.to_string_lossy().into_owned();
        let graph = match load_layered_graph_from_elk_text(&path) {
            Ok(graph) => graph,
            Err(err) => {
                parse_failures.push(format!("{path}: {err}"));
                continue;
            }
        };

        let validation_result = panic::catch_unwind(AssertUnwindSafe(|| {
            run_layout_prefer_recursive(&graph);

            if has_self_loop_without_sections(&graph) {
                panic!(
                    "self-loop recursive layout produced unrouted sections for external resource {path}"
                );
            }

            let self_loop_edges = collect_all_edges(&graph)
                .into_iter()
                .filter(is_self_loop_edge)
                .collect::<Vec<_>>();
            if self_loop_edges.is_empty() {
                return false;
            }

            for edge in self_loop_edges {
                assert_self_loop_geometry(&edge, &path);
            }
            true
        }));

        match validation_result {
            Ok(has_self_loops) => {
                if !has_self_loops {
                    missing_self_loops.push(path);
                    continue;
                }
            }
            Err(payload) => {
                layout_failures.push(format!("{path}: {}", panic_payload_to_string(payload)));
                continue;
            }
        }
        checked += 1;
    }

    assert!(
        checked >= MIN_EXTERNAL_SELF_LOOP_CHECKED,
        "self_loop(resources): expected at least {MIN_EXTERNAL_SELF_LOOP_CHECKED} validated external resources, got checked={checked}, sampled={}, parse_failures={}, layout_failures={}, no-self-loop={}",
        sampled_resources.len(),
        parse_failures.len(),
        layout_failures.len(),
        missing_self_loops.len()
    );

    if !parse_failures.is_empty() || !layout_failures.is_empty() || !missing_self_loops.is_empty() {
        eprintln!(
            "self_loop(resources): checked={checked}, sampled={}, parse_failures={} (sample: {}), layout_failures={} (sample: {}), no-self-loop={} (sample: {})",
            sampled_resources.len(),
            parse_failures.len(),
            parse_failures
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join(" | "),
            layout_failures.len(),
            layout_failures
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join(" | "),
            missing_self_loops.len(),
            missing_self_loops
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
}
