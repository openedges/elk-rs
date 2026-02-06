mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_layout, set_node_property};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkLabelRef, ElkNodeRef};

#[test]
fn issue_701_port_label_placement_cases_layout_without_failures() {
    init_layered_options();

    let cases = [
        (
            "inside",
            "issue_701_port_labels_inside.elkt",
        ),
        (
            "outside",
            "issue_701_port_labels_outside.elkt",
        ),
        (
            "fixed",
            "issue_701_port_labels_fixed.elkt",
        ),
    ];

    for (name, file_name) in cases {
        let path = format!(
            "{}/tests/resources/issues/{file_name}",
            env!("CARGO_MANIFEST_DIR")
        );
        let graph = load_layered_graph_from_elkt(&path).expect("issue_701 resource should load");

        let reference_node =
            find_node_by_identifier(&graph, "reference").expect("reference node should exist");
        set_node_property(
            &reference_node,
            LayeredOptions::NODE_SIZE_CONSTRAINTS,
            EnumSet::of(&[SizeConstraint::PortLabels]),
        );

        run_layout(&graph);

        let (node_w, node_h) = node_size(&reference_node);
        assert!(
            node_w.is_finite() && node_h.is_finite() && node_w >= 0.0 && node_h >= 0.0,
            "invalid node size for placement {name}: ({node_w}, {node_h})"
        );

        let labels = collect_port_labels(&reference_node);
        assert_eq!(labels.len(), 4, "expected one label per port for {name}");
        for label in labels {
            let (x, y) = label_location(&label);
            assert!(
                x.is_finite() && y.is_finite(),
                "non-finite label coordinate for placement {name}"
            );
        }
    }
}

fn collect_port_labels(node: &ElkNodeRef) -> Vec<ElkLabelRef> {
    let ports: Vec<_> = node.borrow_mut().ports().iter().cloned().collect();
    let mut labels = Vec::new();

    for port in ports {
        let port_labels: Vec<_> = {
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

fn node_size(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
}

fn label_location(label: &ElkLabelRef) -> (f64, f64) {
    let mut label_mut = label.borrow_mut();
    let shape = label_mut.shape();
    (shape.x(), shape.y())
}
