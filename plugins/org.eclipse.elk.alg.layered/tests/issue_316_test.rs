mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_node_by_identifier, find_port_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_recursive_layout};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkNodeRef, ElkPortRef};

#[test]
fn issue_316_ports_do_not_overlap_and_hierarchical_labels_fit() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_316_ports_labels.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_316 resource should load");

    set_port_location(
        &find_port_by_identifier(&graph, "parent_p1").expect("parent_p1 should exist"),
        0.0,
        10.0,
    );
    set_port_location(
        &find_port_by_identifier(&graph, "parent_p2").expect("parent_p2 should exist"),
        52.0,
        24.0,
    );
    set_port_location(
        &find_port_by_identifier(&graph, "child_a_p").expect("child_a_p should exist"),
        32.0,
        6.0,
    );
    set_port_location(
        &find_port_by_identifier(&graph, "child_b_p").expect("child_b_p should exist"),
        0.0,
        18.0,
    );

    run_recursive_layout(&graph);

    let all_nodes = collect_nodes(&graph);
    for node in &all_nodes {
        assert_no_port_overlaps(node);
    }

    for node in all_nodes {
        assert_hierarchical_labels_inside_node(&node);
    }

    let parent = find_node_by_identifier(&graph, "parent").expect("parent node should exist");
    assert_hierarchical_labels_inside_node(&parent);
}

fn collect_nodes(root: &ElkNodeRef) -> Vec<ElkNodeRef> {
    let mut result = Vec::new();
    let mut queue = vec![root.clone()];

    while let Some(node) = queue.pop() {
        let children: Vec<ElkNodeRef> = node.borrow_mut().children().iter().cloned().collect();
        for child in children {
            queue.push(child.clone());
            result.push(child);
        }
    }

    result
}

fn assert_no_port_overlaps(node: &ElkNodeRef) {
    let ports: Vec<ElkPortRef> = node.borrow_mut().ports().iter().cloned().collect();
    for (left_index, left) in ports.iter().enumerate() {
        let left_rect = port_rect(left);
        for right in ports.iter().skip(left_index + 1) {
            let right_rect = port_rect(right);
            let overlaps = left_rect.0 < right_rect.0 + right_rect.2
                && left_rect.0 + left_rect.2 > right_rect.0
                && left_rect.1 < right_rect.1 + right_rect.3
                && left_rect.1 + left_rect.3 > right_rect.1;
            assert!(
                !overlaps,
                "port overlap detected: left={left_rect:?} right={right_rect:?}"
            );
        }
    }
}

fn port_rect(port: &ElkPortRef) -> (f64, f64, f64, f64) {
    let mut port_mut = port.borrow_mut();
    let shape = port_mut.connectable().shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}

fn set_port_location(port: &ElkPortRef, x: f64, y: f64) {
    let mut port_mut = port.borrow_mut();
    port_mut.connectable().shape().set_location(x, y);
}

fn assert_hierarchical_labels_inside_node(node: &ElkNodeRef) {
    let (has_children, node_width, node_height, labels) = {
        let mut node_mut = node.borrow_mut();
        let has_children = !node_mut.children().is_empty();
        let shape = node_mut.connectable().shape();
        let labels = shape.graph_element().labels().iter().cloned().collect::<Vec<_>>();
        (has_children, shape.width(), shape.height(), labels)
    };

    if !has_children {
        return;
    }

    for label in labels {
        let mut label_mut = label.borrow_mut();
        let shape = label_mut.shape();
        assert!(
            shape.x() >= 0.0
                && shape.y() >= 0.0
                && shape.x() + shape.width() <= node_width
                && shape.y() + shape.height() <= node_height,
            "hierarchical label outside node: label=({}, {}, {}, {}), node=({}, {})",
            shape.x(),
            shape.y(),
            shape.width(),
            shape.height(),
            node_width,
            node_height
        );
    }
}
