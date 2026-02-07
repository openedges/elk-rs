mod elkt_test_loader;
mod issue_support;

use std::collections::VecDeque;
use std::env;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use elkt_test_loader::load_layered_graph_from_elk_text;
use issue_support::{init_layered_options, run_layout};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkNodeRef, ElkPortRef};

const MIN_EXTERNAL_NODESPACING_CHECKED: usize = 2;
const MAX_EXTERNAL_NODESPACING_SCAN: usize = 24;

fn collect_external_core_nodespacing_resources() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../external/elk-models/tickets/core"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../external/elk-models/tickets/layered"),
    ];
    if let Ok(path) = env::var("ELK_CORE_TICKETS_DIR") {
        roots.push(PathBuf::from(path));
    }

    let mut resources = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        collect_core_ticket_files(&root, &mut resources);
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

fn collect_core_ticket_files(root: &Path, resources: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_core_ticket_files(&path, resources);
            continue;
        }

        let ext = path.extension().and_then(|value| value.to_str());
        if ext != Some("elkt") && ext != Some("elkg") {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if is_nodespacing_ticket_resource(file_name) {
            resources.push(path);
        }
    }
}

fn is_nodespacing_ticket_resource(file_name: &str) -> bool {
    const PREFIXES: [&str; 11] = [
        "056_", "167_", "245_", "269_", "296_", "299_", "405_", "491_", "562_", "596_",
        "701_",
    ];
    PREFIXES.iter().any(|prefix| file_name.starts_with(prefix))
}

fn collect_all_ports(graph: &ElkNodeRef) -> Vec<ElkPortRef> {
    let mut queue = VecDeque::new();
    let mut ports = Vec::new();
    queue.push_back(graph.clone());

    while let Some(node) = queue.pop_front() {
        let node_ports: Vec<_> = node.borrow_mut().ports().iter().cloned().collect();
        ports.extend(node_ports);

        let children: Vec<_> = node.borrow_mut().children().iter().cloned().collect();
        queue.extend(children);
    }

    ports
}

fn assert_finite(value: f64, context: &str) {
    assert!(
        value.is_finite(),
        "non-finite geometry value in {context}: {value}"
    );
}

#[test]
fn common_nodespacing_external_ticket_resources_if_available_have_finite_port_label_geometry() {
    init_layered_options();

    let resources = collect_external_core_nodespacing_resources();
    if resources.is_empty() {
        eprintln!("common_nodespacing(resources): no external core ticket resources found");
        return;
    }

    let sampled_resources = sample_resources_spread(&resources, MAX_EXTERNAL_NODESPACING_SCAN);
    let mut checked = 0usize;
    let mut parse_failures = Vec::new();
    let mut layout_failures = Vec::new();
    let mut no_label_resources = Vec::new();

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
            run_layout(&graph);

            let mut label_count = 0usize;
            for port in collect_all_ports(&graph) {
                let labels: Vec<_> = port
                    .borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect();
                for label_ref in labels {
                    let mut label = label_ref.borrow_mut();
                    let shape = label.shape();
                    assert_finite(shape.x(), &path);
                    assert_finite(shape.y(), &path);
                    assert_finite(shape.width(), &path);
                    assert_finite(shape.height(), &path);
                    assert!(
                        shape.width() >= 0.0 && shape.height() >= 0.0,
                        "label size must be non-negative for resource {path}"
                    );
                    label_count += 1;
                }
            }
            label_count
        }));

        match validation_result {
            Ok(label_count) => {
                if label_count > 0 {
                    checked += 1;
                } else {
                    no_label_resources.push(path);
                }
            }
            Err(payload) => {
                layout_failures.push(format!("{path}: {}", panic_payload_to_string(payload)));
            }
        }
    }

    assert!(
        checked >= MIN_EXTERNAL_NODESPACING_CHECKED,
        "common_nodespacing(resources): expected at least {MIN_EXTERNAL_NODESPACING_CHECKED} validated external resources, got checked={checked}, sampled={}, parse_failures={}, layout_failures={}, no-label={}",
        sampled_resources.len(),
        parse_failures.len(),
        layout_failures.len(),
        no_label_resources.len()
    );

    if !parse_failures.is_empty() || !layout_failures.is_empty() || !no_label_resources.is_empty() {
        eprintln!(
            "common_nodespacing(resources): checked={checked}, sampled={}, parse_failures={} (sample: {}), layout_failures={} (sample: {}), no-label={} (sample: {})",
            sampled_resources.len(),
            parse_failures.len(),
            parse_failures
                .iter()
                .take(2)
                .cloned()
                .collect::<Vec<_>>()
                .join(" | "),
            layout_failures.len(),
            layout_failures
                .iter()
                .take(2)
                .cloned()
                .collect::<Vec<_>>()
                .join(" | "),
            no_label_resources.len(),
            no_label_resources
                .iter()
                .take(2)
                .cloned()
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
}
