mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::load_layered_graph_from_elkt;
use issue_support::init_layered_options;
use issue_support::run_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkGraphElementRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

const OVERLAP_EPSILON: f64 = 0.5;

#[test]
fn test_no_label_overlaps() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/common_nodespacing/inside_port_labels.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path)
        .unwrap_or_else(|err| panic!("inside_port_labels resource should load: {err}"));

    run_layout(&graph);

    let children: Vec<ElkNodeRef> = graph.borrow_mut().children().iter().cloned().collect();
    for node in children {
        let labels = assemble_labels(&node);
        assert_no_overlaps(&labels);
    }
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

fn assert_no_overlaps(labels: &[ElkLabelRef]) {
    for (left_index, left_label) in labels.iter().enumerate() {
        let left = absolute_label_bounds(left_label);
        for right_label in labels.iter().skip(left_index + 1) {
            let right = absolute_label_bounds(right_label);
            assert!(
                !rectangles_overlap(left, right),
                "label overlap detected: left={left:?}, right={right:?}"
            );
        }
    }
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
    left.0 < right.0 + right.2 - OVERLAP_EPSILON
        && left.0 + left.2 > right.0 + OVERLAP_EPSILON
        && left.1 < right.1 + right.3 - OVERLAP_EPSILON
        && left.1 + left.3 > right.1 + OVERLAP_EPSILON
}
