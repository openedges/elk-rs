
use crate::common::elkt_test_loader::{find_edge_by_identifier, load_layered_graph_from_elkt};
use crate::common::issue_support::{init_layered_options, run_layout};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkLabelRef, ElkNodeRef};

#[test]
fn issue_905_edge_label_order_and_alignment_are_stable() {
    init_layered_options();

    let (graph_a, tail_a, center_a, head_a) = load_issue_905_graph();
    configure_label(&tail_a, EdgeLabelPlacement::Tail, 5.0, 10.0);
    configure_label(&center_a, EdgeLabelPlacement::Center, 20.0, 80.0);
    configure_label(&head_a, EdgeLabelPlacement::Head, 35.0, 150.0);

    let (graph_b, tail_b, center_b, head_b) = load_issue_905_graph();
    configure_label(&tail_b, EdgeLabelPlacement::Tail, 5.0, 150.0);
    configure_label(&center_b, EdgeLabelPlacement::Center, 20.0, 10.0);
    configure_label(&head_b, EdgeLabelPlacement::Head, 35.0, 80.0);

    run_layout(&graph_a);
    run_layout(&graph_b);

    let (tail_x_a, tail_y_a) = label_location(&tail_a);
    let (center_x_a, center_y_a) = label_location(&center_a);
    let (head_x_a, head_y_a) = label_location(&head_a);

    let (tail_x_b, tail_y_b) = label_location(&tail_b);
    let (center_x_b, center_y_b) = label_location(&center_b);
    let (head_x_b, head_y_b) = label_location(&head_b);

    for (x, y) in [
        (tail_x_a, tail_y_a),
        (center_x_a, center_y_a),
        (head_x_a, head_y_a),
        (tail_x_b, tail_y_b),
        (center_x_b, center_y_b),
        (head_x_b, head_y_b),
    ] {
        assert!(
            x.is_finite() && y.is_finite(),
            "edge label has non-finite coordinates"
        );
    }

    assert!(
        tail_x_a <= center_x_a && center_x_a <= head_x_a,
        "unexpected edge-label x order (graph A): tail={tail_x_a}, center={center_x_a}, head={head_x_a}"
    );
    assert!(
        tail_x_b <= center_x_b && center_x_b <= head_x_b,
        "unexpected edge-label x order (graph B): tail={tail_x_b}, center={center_x_b}, head={head_x_b}"
    );

    assert!(
        (tail_x_a - tail_x_b).abs() <= 0.5
            && (center_x_a - center_x_b).abs() <= 0.5
            && (head_x_a - head_x_b).abs() <= 0.5,
        "x positions should be stable across input variants: A=({tail_x_a},{center_x_a},{head_x_a}) B=({tail_x_b},{center_x_b},{head_x_b})"
    );
}

fn load_issue_905_graph() -> (ElkNodeRef, ElkLabelRef, ElkLabelRef, ElkLabelRef) {
    let path = format!(
        "{}/tests/resources/issues/issue_905_edge_labels.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path).expect("issue_905 resource should load");
    let edge = find_edge_by_identifier(&graph, "source", "target").expect("main edge should exist");

    let labels: Vec<_> = edge
        .borrow_mut()
        .element()
        .labels()
        .iter()
        .cloned()
        .collect();
    let mut tail = None;
    let mut center = None;
    let mut head = None;

    for label in labels {
        let text = label.borrow().text().to_string();
        match text.as_str() {
            "tail" => tail = Some(label),
            "center" => center = Some(label),
            "head" => head = Some(label),
            _ => {}
        }
    }

    (
        graph,
        tail.expect("tail label should exist"),
        center.expect("center label should exist"),
        head.expect("head label should exist"),
    )
}

fn configure_label(label: &ElkLabelRef, placement: EdgeLabelPlacement, x: f64, y: f64) {
    set_label_property(label, LayeredOptions::EDGE_LABELS_PLACEMENT, placement);
    set_label_location(label, x, y);
}

fn label_location(label: &ElkLabelRef) -> (f64, f64) {
    let mut label_mut = label.borrow_mut();
    let shape = label_mut.shape();
    (shape.x(), shape.y())
}

fn set_label_location(label: &ElkLabelRef, x: f64, y: f64) {
    let mut label_mut = label.borrow_mut();
    label_mut.shape().set_location(x, y);
}

fn set_label_property<T: Clone + Send + Sync + 'static>(
    label: &ElkLabelRef,
    property: &Property<T>,
    value: T,
) {
    let mut label_mut = label.borrow_mut();
    label_mut
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}
