mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_node_by_identifier, find_port_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_recursive_layout};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkNodeRef, ElkPortRef};

#[test]
fn issue_316_ensure_ports_dont_overlap() {
    init_layered_options();

    for graph in load_issue_316_graphs() {
        for node in collect_nodes(&graph) {
            assert_no_port_overlaps(&node);
        }
    }
}

#[test]
fn issue_316_ensure_nodes_big_enough_for_labels() {
    init_layered_options();

    for graph in load_issue_316_graphs() {
        for node in collect_nodes(&graph) {
            assert_hierarchical_labels_inside_node(&node);
        }
        let parent = find_node_by_identifier(&graph, "parent").expect("parent node should exist");
        assert_hierarchical_labels_inside_node(&parent);
    }
}

fn load_issue_316_graphs() -> Vec<ElkNodeRef> {
    const RESOURCES: [&str; 2] = [
        "issue_316_ports_labels.elkt",
        "issue_316_ports_labels_2.elkt",
    ];
    let mut graphs = Vec::with_capacity(RESOURCES.len());

    for resource in RESOURCES {
        let path = format!(
            "{}/tests/resources/issues/{resource}",
            env!("CARGO_MANIFEST_DIR")
        );
        let graph = load_layered_graph_from_elkt(&path)
            .unwrap_or_else(|_| panic!("issue_316 resource {resource} should load"));

        set_port_location_if_present(&graph, "parent_p1", 0.0, 10.0);
        set_port_location_if_present(&graph, "parent_p2", 52.0, 24.0);
        set_port_location_if_present(&graph, "child_a_p", 32.0, 6.0);
        set_port_location_if_present(&graph, "child_b_p", 0.0, 18.0);

        run_recursive_layout(&graph);
        graphs.push(graph);
    }

    graphs
}

fn set_port_location_if_present(graph: &ElkNodeRef, port_id: &str, x: f64, y: f64) {
    if let Some(port) = find_port_by_identifier(graph, port_id) {
        set_port_location(&port, x, y);
    }
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
