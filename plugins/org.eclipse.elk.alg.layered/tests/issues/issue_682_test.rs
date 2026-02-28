
use crate::common::elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use crate::common::issue_support::{init_layered_options, run_layout, set_node_property};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{Direction, SizeConstraint};
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkLabelRef, ElkNodeRef};

const EPSILON: f64 = 1.0e-5;

#[test]
fn issue_682_node_label_padding_is_respected_in_all_directions() {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/issues/issue_682_node_labels_padding.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let directions = [
        Direction::Right,
        Direction::Down,
        Direction::Up,
        Direction::Left,
    ];

    for direction in directions {
        let graph = load_layered_graph_from_elkt(&path).expect("issue_682 resource should load");
        set_node_property(&graph, CoreOptions::DIRECTION, direction);

        let parent = find_node_by_identifier(&graph, "parent").expect("parent node should exist");
        set_node_property(
            &parent,
            LayeredOptions::NODE_SIZE_CONSTRAINTS,
            EnumSet::of(&[SizeConstraint::NodeLabels]),
        );

        run_layout(&graph);

        let label = first_node_label(&parent);
        let (label_x, label_y, label_width, label_height) = label_bounds(&label);
        let parent_width = node_width(&parent);

        assert!(
            (label_width - 23.0).abs() <= EPSILON
                && (label_height - 22.0).abs() <= EPSILON
                && label_x.is_finite()
                && label_y.is_finite()
                && parent_width.is_finite()
                && parent_width >= 0.0,
            "unexpected values for {:?}: label=({label_x},{label_y},{label_width},{label_height}), parent_width={parent_width}",
            direction
        );
    }
}

fn first_node_label(node: &ElkNodeRef) -> ElkLabelRef {
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .labels()
        .get(0)
        .expect("expected at least one node label")
}

fn node_width(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().width()
}

fn label_bounds(label: &ElkLabelRef) -> (f64, f64, f64, f64) {
    let mut label_mut = label.borrow_mut();
    let shape = label_mut.shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}
