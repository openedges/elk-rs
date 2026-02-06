mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_layout, set_node_property};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    PortLabelPlacement, PortSide, SizeConstraint,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, EnumSet};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkGraphElementRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

const COORDINATE_FUZZINESS: f64 = 0.5;

#[derive(Clone)]
struct PortLabelCase {
    port_id: String,
    side: PortSide,
    label: ElkLabelRef,
}

#[test]
fn issue_701_test_inside_labels() {
    init_layered_options();

    let graph = load_issue_701_graph("issue_701_port_labels_inside.elkt");
    let reference_node = find_reference_node(&graph);
    let labels = collect_port_label_cases(&reference_node);
    let placement = port_label_placement(&reference_node);

    assert_eq!(labels.len(), 4, "inside case should have four labeled ports");
    assert_no_label_overlaps(&labels, "inside");
    assert!(
        placement.contains(&PortLabelPlacement::Inside),
        "inside case should expose inside placement option"
    );
    assert!(
        PortLabelPlacement::is_valid(&placement),
        "inside placement option should be valid"
    );
    assert_labels_aligned_with_port_side(&reference_node, &labels, "inside");
}

#[test]
fn issue_701_test_outside_labels() {
    init_layered_options();

    let graph = load_issue_701_graph("issue_701_port_labels_outside.elkt");
    let reference_node = find_reference_node(&graph);
    let labels = collect_port_label_cases(&reference_node);
    let placement = port_label_placement(&reference_node);

    assert_eq!(labels.len(), 4, "outside case should have four labeled ports");
    assert_no_label_overlaps(&labels, "outside");
    assert!(
        placement.contains(&PortLabelPlacement::Outside),
        "outside case should expose outside placement option"
    );
    assert!(
        PortLabelPlacement::is_valid(&placement),
        "outside placement option should be valid"
    );
    assert_labels_aligned_with_port_side(&reference_node, &labels, "outside");
}

#[test]
fn issue_701_test_mix_inside_and_outside_labels() {
    init_layered_options();

    let inside_graph = load_issue_701_graph("issue_701_port_labels_inside.elkt");
    let outside_graph = load_issue_701_graph("issue_701_port_labels_outside.elkt");

    let inside_reference = find_reference_node(&inside_graph);
    let outside_reference = find_reference_node(&outside_graph);
    let inside_labels = collect_port_label_cases(&inside_reference);
    let outside_labels = collect_port_label_cases(&outside_reference);
    let inside_placement = port_label_placement(&inside_reference);
    let outside_placement = port_label_placement(&outside_reference);

    assert_no_label_overlaps(&inside_labels, "mix_inside");
    assert_no_label_overlaps(&outside_labels, "mix_outside");
    assert_labels_aligned_with_port_side(&inside_reference, &inside_labels, "mix_inside");
    assert_labels_aligned_with_port_side(&outside_reference, &outside_labels, "mix_outside");
    assert!(
        inside_placement.contains(&PortLabelPlacement::Inside),
        "mix-inside placement should include inside"
    );
    assert!(
        outside_placement.contains(&PortLabelPlacement::Outside),
        "mix-outside placement should include outside"
    );
    assert!(
        inside_placement != outside_placement,
        "inside/outside mix inputs should keep distinct placement sets"
    );
}

#[test]
fn issue_701_test_fixed_inside_labels() {
    init_layered_options();

    let graph = load_issue_701_graph("issue_701_port_labels_fixed.elkt");
    let reference_node = find_reference_node(&graph);
    let labels = collect_port_label_cases(&reference_node);
    let placement = port_label_placement(&reference_node);

    assert_eq!(labels.len(), 4, "fixed-inside case should have four labeled ports");
    assert_no_label_overlaps(&labels, "fixed_inside");
    assert!(
        PortLabelPlacement::is_fixed(&placement),
        "fixed case should expose fixed placement option"
    );

    let node_bounds = node_absolute_bounds(&reference_node);
    let inside_count = count_labels_intersecting_node(node_bounds, &labels);
    assert!(
        inside_count > 0,
        "fixed labels should keep at least one label in/overlapping the node area"
    );
}

#[test]
fn issue_701_test_fixed_outside_labels() {
    init_layered_options();

    let graph = load_issue_701_graph("issue_701_port_labels_fixed.elkt");
    let reference_node = find_reference_node(&graph);
    let labels = collect_port_label_cases(&reference_node);
    let placement = port_label_placement(&reference_node);

    assert_eq!(labels.len(), 4, "fixed-outside case should have four labeled ports");
    assert_no_label_overlaps(&labels, "fixed_outside");
    assert!(
        PortLabelPlacement::is_fixed(&placement),
        "fixed case should expose fixed placement option"
    );

    let node_bounds = node_absolute_bounds(&reference_node);
    let outside_count = count_outside_labels(node_bounds, &labels);
    assert!(
        outside_count > 0,
        "fixed labels should keep at least one label outside the node border"
    );
}

#[test]
fn issue_701_test_fixed_mix_inside_and_outside_labels() {
    init_layered_options();

    let graph = load_issue_701_graph("issue_701_port_labels_fixed.elkt");
    let reference_node = find_reference_node(&graph);
    let labels = collect_port_label_cases(&reference_node);
    let placement = port_label_placement(&reference_node);

    assert_eq!(labels.len(), 4, "fixed-mix case should have four labeled ports");
    assert_no_label_overlaps(&labels, "fixed_mix");
    assert!(
        PortLabelPlacement::is_fixed(&placement),
        "fixed case should expose fixed placement option"
    );

    let node_bounds = node_absolute_bounds(&reference_node);
    let inside_count = count_labels_intersecting_node(node_bounds, &labels);
    let outside_count = count_outside_labels(node_bounds, &labels);
    assert!(
        inside_count > 0 && outside_count > 0,
        "fixed mix should have labels inside/overlapping and outside \
         (inside={inside_count}, outside={outside_count})"
    );
}

fn load_issue_701_graph(resource: &str) -> ElkNodeRef {
    let path = format!(
        "{}/tests/resources/issues/{resource}",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path)
        .unwrap_or_else(|_| panic!("issue_701 resource {resource} should load"));

    let reference_node =
        find_node_by_identifier(&graph, "reference").expect("reference node should exist");
    set_node_property(
        &reference_node,
        LayeredOptions::NODE_SIZE_CONSTRAINTS,
        EnumSet::of(&[SizeConstraint::PortLabels]),
    );

    run_layout(&graph);
    graph
}

fn find_reference_node(graph: &ElkNodeRef) -> ElkNodeRef {
    find_node_by_identifier(graph, "reference").expect("reference node should exist")
}

fn collect_port_label_cases(node: &ElkNodeRef) -> Vec<PortLabelCase> {
    let ports: Vec<ElkPortRef> = node.borrow_mut().ports().iter().cloned().collect();
    let mut result = Vec::new();

    for port in ports {
        let (port_id, side, label) = {
            let mut port_mut = port.borrow_mut();
            let shape = port_mut.connectable().shape();
            let graph_element = shape.graph_element();
            let port_id = graph_element
                .identifier()
                .map(ToString::to_string)
                .expect("port identifier should be present");
            let side = graph_element
                .properties_mut()
                .get_property(LayeredOptions::PORT_SIDE)
                .expect("port side should be set");
            let label = graph_element
                .labels()
                .get(0)
                .unwrap_or_else(|| panic!("port {port_id} should have a label"));
            (port_id, side, label)
        };
        result.push(PortLabelCase {
            port_id,
            side,
            label,
        });
    }

    result
}

fn port_label_placement(node: &ElkNodeRef) -> EnumSet<PortLabelPlacement> {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
        .expect("port labels placement should be set")
}

fn node_absolute_bounds(node: &ElkNodeRef) -> (f64, f64, f64, f64) {
    let (width, height) = {
        let mut node_mut = node.borrow_mut();
        let shape = node_mut.connectable().shape();
        (shape.width(), shape.height())
    };
    let pos = ElkUtil::absolute_position(&ElkGraphElementRef::Node(node.clone()))
        .expect("node absolute position should exist");
    (pos.x, pos.y, width, height)
}

fn label_absolute_bounds(label: &ElkLabelRef) -> (f64, f64, f64, f64) {
    let (width, height) = {
        let mut label_mut = label.borrow_mut();
        let shape = label_mut.shape();
        (shape.width(), shape.height())
    };
    let pos = ElkUtil::absolute_position(&ElkGraphElementRef::Label(label.clone()))
        .expect("label absolute position should exist");
    (pos.x, pos.y, width, height)
}

fn rectangles_overlap(left: (f64, f64, f64, f64), right: (f64, f64, f64, f64)) -> bool {
    left.0 < right.0 + right.2 - COORDINATE_FUZZINESS
        && left.0 + left.2 > right.0 + COORDINATE_FUZZINESS
        && left.1 < right.1 + right.3 - COORDINATE_FUZZINESS
        && left.1 + left.3 > right.1 + COORDINATE_FUZZINESS
}

fn assert_no_label_overlaps(labels: &[PortLabelCase], context: &str) {
    let bounds: Vec<_> = labels
        .iter()
        .map(|entry| (entry.port_id.clone(), label_absolute_bounds(&entry.label)))
        .collect();
    for (left_index, (left_id, left_bounds)) in bounds.iter().enumerate() {
        for (right_id, right_bounds) in bounds.iter().skip(left_index + 1) {
            assert!(
                !rectangles_overlap(*left_bounds, *right_bounds),
                "label overlap in {context}: {left_id}={left_bounds:?}, {right_id}={right_bounds:?}"
            );
        }
    }
}

fn assert_labels_aligned_with_port_side(node: &ElkNodeRef, labels: &[PortLabelCase], context: &str) {
    let node_bounds = node_absolute_bounds(node);
    let node_mid_x = node_bounds.0 + node_bounds.2 / 2.0;
    let node_mid_y = node_bounds.1 + node_bounds.3 / 2.0;

    for label_case in labels {
        let label_bounds = label_absolute_bounds(&label_case.label);
        let label_mid_x = label_bounds.0 + label_bounds.2 / 2.0;
        let label_mid_y = label_bounds.1 + label_bounds.3 / 2.0;
        assert!(
            match label_case.side {
                PortSide::West => label_mid_x <= node_mid_x + COORDINATE_FUZZINESS,
                PortSide::East => label_mid_x >= node_mid_x - COORDINATE_FUZZINESS,
                PortSide::North => label_mid_y <= node_mid_y + COORDINATE_FUZZINESS,
                PortSide::South => label_mid_y >= node_mid_y - COORDINATE_FUZZINESS,
                _ => false,
            },
            "label should stay on the expected side in {context}: port={} side={:?} label={:?} node={:?}",
            label_case.port_id,
            label_case.side,
            label_bounds,
            node_bounds
        );
    }
}

fn is_label_outside_for_side(
    node_bounds: (f64, f64, f64, f64),
    label_bounds: (f64, f64, f64, f64),
    side: PortSide,
) -> bool {
    match side {
        PortSide::West => label_bounds.0 + label_bounds.2 <= node_bounds.0 + COORDINATE_FUZZINESS,
        PortSide::East => {
            label_bounds.0 >= node_bounds.0 + node_bounds.2 - COORDINATE_FUZZINESS
        }
        PortSide::North => label_bounds.1 + label_bounds.3 <= node_bounds.1 + COORDINATE_FUZZINESS,
        PortSide::South => {
            label_bounds.1 >= node_bounds.1 + node_bounds.3 - COORDINATE_FUZZINESS
        }
        _ => false,
    }
}

fn count_labels_intersecting_node(
    node_bounds: (f64, f64, f64, f64),
    labels: &[PortLabelCase],
) -> usize {
    labels
        .iter()
        .map(|entry| label_absolute_bounds(&entry.label))
        .filter(|bounds| rectangles_overlap(*bounds, node_bounds))
        .count()
}

fn count_outside_labels(node_bounds: (f64, f64, f64, f64), labels: &[PortLabelCase]) -> usize {
    labels
        .iter()
        .filter(|entry| {
            is_label_outside_for_side(
                node_bounds,
                label_absolute_bounds(&entry.label),
                entry.side,
            )
        })
        .count()
}
