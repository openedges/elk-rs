use crate::elkt_test_loader::load_layered_graph_from_elkt;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, ElkUtil};
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkGraphElementRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};
use std::collections::VecDeque;
use std::env;
use std::path::PathBuf;

const OVERLAP_EPSILON: f64 = 0.5;
const LABEL_CHAR_WIDTH: f64 = 6.0;
const LABEL_HEIGHT: f64 = 10.0;

type NodeDebugInfo = (f64, f64, Vec<(f64, f64, f64, f64)>, Vec<ElkLabelRef>);

fn run_layout(graph: &ElkNodeRef) {
    let mut provider = LayeredLayoutProvider::new();
    provider.layout(graph, &mut BasicProgressMonitor::new());
}

fn apply_default_port_configuration(graph: &ElkNodeRef) {
    let children: Vec<ElkNodeRef> = graph.borrow_mut().children().iter().cloned().collect();
    let mut queue: VecDeque<ElkNodeRef> = VecDeque::from(children);
    while let Some(node) = queue.pop_front() {
        let (children, ports): (Vec<ElkNodeRef>, Vec<ElkPortRef>) = {
            let mut node_mut = node.borrow_mut();
            let children = node_mut.children().iter().cloned().collect();
            let ports = node_mut.ports().iter().cloned().collect();
            (children, ports)
        };
        for port in ports {
            ElkUtil::configure_with_default_values(&port);
        }
        queue.extend(children);
    }
}

fn ensure_label_sizes(graph: &ElkNodeRef) {
    let mut queue: VecDeque<ElkNodeRef> = VecDeque::from([graph.clone()]);
    while let Some(node) = queue.pop_front() {
        let (children, node_labels, ports): (Vec<ElkNodeRef>, Vec<ElkLabelRef>, Vec<ElkPortRef>) = {
            let mut node_mut = node.borrow_mut();
            let labels = node_mut
                .connectable()
                .shape()
                .graph_element()
                .labels()
                .iter()
                .cloned()
                .collect();
            let ports = node_mut.ports().iter().cloned().collect();
            let children = node_mut.children().iter().cloned().collect();
            (children, labels, ports)
        };

        for label in node_labels {
            set_label_size_if_missing(&label);
        }

        for port in ports {
            let port_labels: Vec<ElkLabelRef> = {
                let mut port_mut = port.borrow_mut();
                port_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect()
            };
            for label in port_labels {
                set_label_size_if_missing(&label);
            }
        }

        queue.extend(children);
    }
}

fn set_label_size_if_missing(label: &ElkLabelRef) {
    let (width, height, text) = {
        let mut label_mut = label.borrow_mut();
        let shape = label_mut.shape();
        (shape.width(), shape.height(), label_mut.text().to_string())
    };
    if width != 0.0 || height != 0.0 || text.is_empty() {
        return;
    }
    let width = (text.chars().count() as f64) * LABEL_CHAR_WIDTH;
    let height = LABEL_HEIGHT;
    label
        .borrow_mut()
        .shape()
        .set_dimensions(width.max(1.0), height);
}

fn assemble_labels(node: &ElkNodeRef) -> Vec<ElkLabelRef> {
    let (mut labels, ports): (Vec<ElkLabelRef>, Vec<ElkPortRef>) = {
        let mut node_mut = node.borrow_mut();
        let graph_element = node_mut.connectable().shape().graph_element();
        let labels = graph_element.labels().iter().cloned().collect();
        let ports = node_mut.ports().iter().cloned().collect();
        (labels, ports)
    };

    for port in ports {
        let port_labels: Vec<ElkLabelRef> = {
            let mut port_mut = port.borrow_mut();
            port_mut
                .connectable()
                .shape()
                .graph_element()
                .labels()
                .iter()
                .cloned()
                .collect()
        };
        labels.extend(port_labels);
    }

    labels
}

fn absolute_label_bounds(label: &ElkLabelRef) -> (f64, f64, f64, f64) {
    let (width, height) = {
        let mut label_mut = label.borrow_mut();
        let shape = label_mut.shape();
        (shape.width(), shape.height())
    };
    let abs = ElkUtil::absolute_position(&ElkGraphElementRef::Label(label.clone()))
        .expect("label absolute position should be available");
    (abs.x, abs.y, width, height)
}

fn rectangles_overlap(left: (f64, f64, f64, f64), right: (f64, f64, f64, f64)) -> bool {
    let (lx, ly, lw, lh) = left;
    let (rx, ry, rw, rh) = right;
    let horizontal = lx + lw - OVERLAP_EPSILON > rx && lx + OVERLAP_EPSILON < rx + rw;
    let vertical = ly + lh - OVERLAP_EPSILON > ry && ly + OVERLAP_EPSILON < ry + rh;
    horizontal && vertical
}

fn assert_no_overlaps(labels: &[ElkLabelRef]) {
    for (left_index, left_label) in labels.iter().enumerate() {
        let left = absolute_label_bounds(left_label);
        for right_label in labels.iter().skip(left_index + 1) {
            let right = absolute_label_bounds(right_label);
            assert!(
                !rectangles_overlap(left, right),
                "label overlap detected: left={:?}, right={:?}",
                label_debug(left_label),
                label_debug(right_label)
            );
        }
    }
}

fn maybe_debug_labels(node: &ElkNodeRef) {
    let debug = env::var("INSIDE_DEBUG_PRINT").is_ok();
    if !debug {
        return;
    }

    let (width, height, node_padding, node_labels): NodeDebugInfo = {
        let mut node_mut = node.borrow_mut();
        let shape = node_mut.connectable().shape();
        let size = (shape.width(), shape.height());
        let labels: Vec<_> = shape.graph_element().labels().iter().cloned().collect();
        let padding = shape
            .graph_element()
            .properties_mut()
            .get_property(CoreOptions::PADDING)
            .unwrap_or_default();
        (
            size.0,
            size.1,
            vec![(padding.left, padding.top, padding.right, padding.bottom)],
            labels,
        )
    };

    println!(
        "node size=({:.2}, {:.2}) padding={:?}",
        width, height, node_padding
    );

    for label in &node_labels {
        println!("  {:?}", label_debug(label));
    }
}

fn label_debug(label: &ElkLabelRef) -> (String, (f64, f64, f64, f64)) {
    let text = label.borrow().text().to_string();
    let bounds = absolute_label_bounds(label);
    (text, bounds)
}

#[test]
fn test_no_label_overlaps() {
    initialize_plain_java_layout();

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../external/elk-models/tests/core/node_size/inside_port_labels.elkt");
    let path = path.to_string_lossy().into_owned();
    let graph = load_layered_graph_from_elkt(&path)
        .unwrap_or_else(|err| panic!("inside_port_labels resource should load: {err}"));

    apply_default_port_configuration(&graph);
    ensure_label_sizes(&graph);
    run_layout(&graph);
    if env::var("INSIDE_DEBUG_PRINT").is_ok() {
        let nodes: Vec<_> = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().iter().cloned().collect()
        };
        for node in &nodes {
            maybe_debug_labels(node);
            let port_labels: Vec<ElkLabelRef> = {
                let mut node_mut = node.borrow_mut();
                let ports = node_mut.ports().iter().cloned().collect::<Vec<_>>();
                let mut result = Vec::new();
                for port in ports {
                    let mut port_mut = port.borrow_mut();
                    result.extend(
                        port_mut
                            .connectable()
                            .shape()
                            .graph_element()
                            .labels()
                            .iter()
                            .cloned(),
                    );
                }
                result
            };
            println!("  port labels count={}", port_labels.len());
            for label in port_labels {
                println!("    port label {:?}", label_debug(&label));
            }
        }
    }

    let children: Vec<ElkNodeRef> = graph.borrow_mut().children().iter().cloned().collect();
    for node in children {
        let labels = assemble_labels(&node);
        assert_no_overlaps(&labels);
    }
}
