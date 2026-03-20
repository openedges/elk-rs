
use crate::common::elkt_test_loader::{
    find_node_by_identifier, find_port_by_identifier, load_layered_graph_from_elkt,
};
use crate::common::issue_support::{init_layered_options, run_layout};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{PortLabelPlacement, PortSide};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkNodeRef, ElkPortRef};

const EPSILON: f64 = 1e-6;

#[test]
fn test_outside_two_default() {
    let graph = load_variants_graph();
    assert_above_or_left(port(&graph, "o2d_p0"));
    assert_below_or_right(port(&graph, "o2d_p1"));
}

#[test]
fn test_outside_two_same_side() {
    let graph = load_variants_graph();
    assert_below_or_right(port(&graph, "o2s_p0"));
    assert_below_or_right(port(&graph, "o2s_p1"));
}

#[test]
fn test_outside_two_next_to_port() {
    let graph = load_variants_graph();
    assert_centered(port(&graph, "o2n_p0"));
    assert_centered(port(&graph, "o2n_p1"));
}

#[test]
fn test_outside_three_default() {
    let graph = load_variants_graph();
    assert_below_or_right(port(&graph, "o3d_p0"));
    assert_below_or_right(port(&graph, "o3d_p1"));
    assert_below_or_right(port(&graph, "o3d_p2"));
}

#[test]
fn test_outside_three_space_efficient() {
    let graph = load_variants_graph();
    assert_above_or_left(port(&graph, "o3s_p0"));
    assert_below_or_right(port(&graph, "o3s_p1"));
    assert_below_or_right(port(&graph, "o3s_p2"));
}

#[test]
fn test_inside_two_default() {
    let graph = load_variants_graph();
    assert_centered(port(&graph, "i2d_p0"));
    assert_centered(port(&graph, "i2d_p1"));
}

#[test]
fn test_inside_two_default_hierarchical() {
    let graph = load_variants_graph();
    assert_below_or_right(port(&graph, "i2dh_p0"));
    assert_below_or_right(port(&graph, "i2dh_p1"));
}

#[test]
fn test_inside_two_next_to_port_hierarchical() {
    let graph = load_variants_graph();
    assert_centered(port(&graph, "i2nh_p0"));
    assert_centered(port(&graph, "i2nh_p1"));
}

#[test]
fn test_inside_two_with_one_edge() {
    let graph = load_variants_graph();
    assert_below_or_right(port(&graph, "i2e_p0"));
    assert_below_or_right(port(&graph, "i2e_p1"));
}

#[test]
fn test_inside_two_with_one_edge_next_to_port() {
    let graph = load_variants_graph();
    assert_node_has_placement(
        &graph,
        "inside_two_with_one_edge_next_to_port",
        PortLabelPlacement::Inside,
    );
    assert_centered(port(&graph, "i2en_p0"));
    let p1 = port(&graph, "i2en_p1");
    assert_has_incident_edge(&p1);
    assert_below_or_right(p1);
}

#[test]
fn test_inside_three_with_one_edge() {
    let graph = load_variants_graph();
    assert_below_or_right(port(&graph, "i3e_p0"));
    assert_below_or_right(port(&graph, "i3e_p1"));
    assert_below_or_right(port(&graph, "i3e_p2"));
}

#[test]
fn test_inside_three_with_one_edge_next_to_port() {
    let graph = load_variants_graph();
    assert_node_has_placement(
        &graph,
        "inside_three_with_one_edge_next_to_port",
        PortLabelPlacement::Inside,
    );
    assert_centered(port(&graph, "i3en_p0"));
    let p1 = port(&graph, "i3en_p1");
    assert_has_incident_edge(&p1);
    assert_below_or_right(p1);
    assert_centered(port(&graph, "i3en_p2"));
}

#[test]
fn test_outside_two_default_for_west_north_south_splits_label_sides() {
    let graph = load_side_variants_graph();
    for node_id in [
        "outside_two_default_west",
        "outside_two_default_north",
        "outside_two_default_south",
    ] {
        let ports = ports_of_node(&graph, node_id);
        assert_eq!(
            ports.len(),
            2,
            "node {node_id} should contain exactly 2 ports"
        );
        let p0 = label_axis_position(&ports[0]);
        let p1 = label_axis_position(&ports[1]);
        assert!(
            p0 * p1 < 0.0,
            "node {node_id} should place one label on each side of the port axis, got p0={p0}, p1={p1}"
        );
    }
}

#[test]
fn test_inside_two_with_one_edge_next_to_port_for_west_north_south() {
    let graph = load_side_variants_graph();
    for node_id in [
        "inside_two_with_one_edge_next_to_port_west",
        "inside_two_with_one_edge_next_to_port_north",
        "inside_two_with_one_edge_next_to_port_south",
    ] {
        assert_node_has_placement(&graph, node_id, PortLabelPlacement::Inside);
        let ports = ports_of_node(&graph, node_id);
        assert_eq!(
            ports.len(),
            2,
            "node {node_id} should contain exactly 2 ports"
        );
        // With full process() (Java parity), port labels are positioned by the cell
        // system. Verify labels don't overlap (the key invariant).
        assert_no_label_overlap_for_ports(&ports, node_id);
    }
}

#[test]
fn test_outside_three_space_efficient_for_west_north_south_uses_both_sides() {
    let graph = load_side_variants_graph();
    for node_id in [
        "outside_three_space_efficient_west",
        "outside_three_space_efficient_north",
        "outside_three_space_efficient_south",
    ] {
        let ports = ports_of_node(&graph, node_id);
        assert_eq!(
            ports.len(),
            3,
            "node {node_id} should contain exactly 3 ports"
        );
        let axis_positions: Vec<f64> = ports.iter().map(label_axis_position).collect();
        let has_positive = axis_positions.iter().any(|p| *p > 0.0);
        let has_negative = axis_positions.iter().any(|p| *p < 0.0);
        assert!(
            has_positive && has_negative,
            "space-efficient outside labels should split sides for node {node_id}, got {:?}",
            axis_positions
        );
    }
}

#[test]
fn test_inside_three_with_one_edge_next_to_port_for_west_north_south() {
    let graph = load_side_variants_graph();
    for node_id in [
        "inside_three_with_one_edge_next_to_port_west",
        "inside_three_with_one_edge_next_to_port_north",
        "inside_three_with_one_edge_next_to_port_south",
    ] {
        assert_node_has_placement(&graph, node_id, PortLabelPlacement::Inside);
        let ports = ports_of_node(&graph, node_id);
        assert_eq!(
            ports.len(),
            3,
            "node {node_id} should contain exactly 3 ports"
        );
        let connected_count = ports.iter().filter(|port| has_incident_edge(port)).count();
        assert_eq!(
            connected_count, 1,
            "node {node_id} should have exactly one connected port"
        );
        // With full process() (Java parity), port labels are positioned by the cell
        // system. Verify labels don't overlap (the key invariant).
        assert_no_label_overlap_for_ports(&ports, node_id);
    }
}

#[test]
fn test_inside_constrained_north_labels_do_not_overlap_and_stay_inside_bounds() {
    let graph = load_side_variants_graph();
    let node_id = "inside_constrained_north";
    let node_width = node_width(&graph, node_id);
    let ports = ports_of_node(&graph, node_id);
    assert_eq!(
        ports.len(),
        3,
        "node {node_id} should contain exactly 3 ports"
    );

    assert_no_label_overlap_for_ports(&ports, node_id);
    for port in &ports {
        let (x, _y, w, _h) = label_rect(port);
        assert!(
            x >= -EPSILON && x + w <= node_width + EPSILON,
            "inside constrained north label should stay inside node width for node {node_id}, rect=({x}, {w}), node_width={node_width}"
        );
    }
}

#[test]
fn test_inside_constrained_south_labels_do_not_overlap_and_stay_inside_bounds() {
    let graph = load_side_variants_graph();
    let node_id = "inside_constrained_south";
    let node_width = node_width(&graph, node_id);
    let ports = ports_of_node(&graph, node_id);
    assert_eq!(
        ports.len(),
        3,
        "node {node_id} should contain exactly 3 ports"
    );

    assert_no_label_overlap_for_ports(&ports, node_id);
    for port in &ports {
        let (x, _y, w, _h) = label_rect(port);
        assert!(
            x >= -EPSILON && x + w <= node_width + EPSILON,
            "inside constrained south label should stay inside node width for node {node_id}, rect=({x}, {w}), node_width={node_width}"
        );
    }
}

fn load_variants_graph() -> org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/common_nodespacing/port_label_placement_variants.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path)
        .unwrap_or_else(|err| panic!("port label variants resource should load: {err}"));
    run_layout(&graph);
    graph
}

fn load_side_variants_graph() -> org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef {
    init_layered_options();

    let path = format!(
        "{}/tests/resources/common_nodespacing/port_label_placement_side_variants.elkt",
        env!("CARGO_MANIFEST_DIR")
    );
    let graph = load_layered_graph_from_elkt(&path)
        .unwrap_or_else(|err| panic!("port label side variants resource should load: {err}"));
    run_layout(&graph);
    graph
}

fn port(
    graph: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef,
    identifier: &str,
) -> ElkPortRef {
    find_port_by_identifier(graph, identifier)
        .unwrap_or_else(|| panic!("port {identifier} should exist"))
}

fn ports_of_node(graph: &ElkNodeRef, node_id: &str) -> Vec<ElkPortRef> {
    let node = find_node_by_identifier(graph, node_id)
        .unwrap_or_else(|| panic!("node {node_id} should exist"));
    let ports: Vec<_> = node.borrow_mut().ports().iter().cloned().collect();
    ports
}

fn node_width(graph: &ElkNodeRef, node_id: &str) -> f64 {
    let node = find_node_by_identifier(graph, node_id)
        .unwrap_or_else(|| panic!("node {node_id} should exist"));
    let width = node.borrow_mut().connectable().shape().width();
    width
}

fn label_rect(port: &ElkPortRef) -> (f64, f64, f64, f64) {
    let mut port_mut = port.borrow_mut();
    let shape = port_mut.connectable().shape();
    let labels = shape.graph_element().labels();
    let label = labels.get(0).expect("port should have one label");
    let mut label_mut = label.borrow_mut();
    let label_shape = label_mut.shape();
    (
        shape.x() + label_shape.x(),
        shape.y() + label_shape.y(),
        label_shape.width(),
        label_shape.height(),
    )
}

fn assert_no_label_overlap_for_ports(ports: &[ElkPortRef], node_id: &str) {
    let rects = ports.iter().map(label_rect).collect::<Vec<_>>();
    for left_index in 0..rects.len() {
        for right_index in (left_index + 1)..rects.len() {
            let (lx, ly, lw, lh) = rects[left_index];
            let (rx, ry, rw, rh) = rects[right_index];
            let horizontal_overlap = lx < rx + rw - EPSILON && lx + lw > rx + EPSILON;
            let vertical_overlap = ly < ry + rh - EPSILON && ly + lh > ry + EPSILON;
            assert!(
                !(horizontal_overlap && vertical_overlap),
                "labels should not overlap for node {node_id}, left={:?}, right={:?}",
                rects[left_index],
                rects[right_index]
            );
        }
    }
}

fn assert_below_or_right(port: ElkPortRef) {
    let label_position = label_axis_position(&port);
    let (incoming, outgoing) = {
        let mut port_mut = port.borrow_mut();
        (
            port_mut.connectable().incoming_edges().len(),
            port_mut.connectable().outgoing_edges().len(),
        )
    };
    assert!(
        label_position > 0.0,
        "expected label below/right, got axis position {label_position}, incoming={incoming}, outgoing={outgoing}"
    );
}

fn assert_above_or_left(port: ElkPortRef) {
    let label_position = label_axis_position(&port);
    assert!(
        label_position < 0.0,
        "expected label above/left, got axis position {label_position}"
    );
}

fn assert_centered(port: ElkPortRef) {
    let (port_side, port_width, port_height, label_x, label_y, label_width, label_height) = {
        let mut port_mut = port.borrow_mut();
        let shape = port_mut.connectable().shape();
        let graph_element = shape.graph_element();
        let port_side = graph_element
            .properties_mut()
            .get_property(CoreOptions::PORT_SIDE)
            .expect("port side should be set");
        let label = graph_element
            .labels()
            .get(0)
            .expect("port should have one label");
        let mut label_mut = label.borrow_mut();
        let label_shape = label_mut.shape();
        (
            port_side,
            shape.width(),
            shape.height(),
            label_shape.x(),
            label_shape.y(),
            label_shape.width(),
            label_shape.height(),
        )
    };

    if matches!(port_side, PortSide::East | PortSide::West) {
        let port_center = port_height / 2.0;
        let label_center = label_y + label_height / 2.0;
        assert!(
            (label_center - port_center).abs() <= EPSILON,
            "expected centered label on Y axis, got label_center={label_center}, port_center={port_center}"
        );
    } else {
        let port_center = port_width / 2.0;
        let label_center = label_x + label_width / 2.0;
        assert!(
            (label_center - port_center).abs() <= EPSILON,
            "expected centered label on X axis, got label_center={label_center}, port_center={port_center}"
        );
    }
}

fn label_axis_position(port: &ElkPortRef) -> f64 {
    let mut port_mut = port.borrow_mut();
    let shape = port_mut.connectable().shape();
    let graph_element = shape.graph_element();
    let port_side = graph_element
        .properties_mut()
        .get_property(CoreOptions::PORT_SIDE)
        .expect("port side should be set");
    let label = graph_element
        .labels()
        .get(0)
        .expect("port should have one label");
    let mut label_mut = label.borrow_mut();
    let label_shape = label_mut.shape();

    if matches!(port_side, PortSide::East | PortSide::West) {
        label_shape.y()
    } else {
        label_shape.x()
    }
}

fn assert_has_incident_edge(port: &ElkPortRef) {
    let (incoming, outgoing) = incident_edge_counts(port);
    assert!(
        incoming + outgoing > 0,
        "expected at least one incident edge, got incoming={incoming}, outgoing={outgoing}"
    );
}

fn has_incident_edge(port: &ElkPortRef) -> bool {
    let (incoming, outgoing) = incident_edge_counts(port);
    incoming + outgoing > 0
}

fn incident_edge_counts(port: &ElkPortRef) -> (usize, usize) {
    let (incoming, outgoing) = {
        let mut port_mut = port.borrow_mut();
        (
            port_mut.connectable().incoming_edges().len(),
            port_mut.connectable().outgoing_edges().len(),
        )
    };
    (incoming, outgoing)
}

fn assert_node_has_placement(graph: &ElkNodeRef, node_id: &str, placement: PortLabelPlacement) {
    let node = crate::common::elkt_test_loader::find_node_by_identifier(graph, node_id)
        .unwrap_or_else(|| panic!("node {node_id} should exist"));
    let placements = node
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
        .unwrap_or_default();
    assert!(
        placements.contains(&placement),
        "node {node_id} should contain placement {:?}, actual {:?}",
        placement,
        placements
    );
}
