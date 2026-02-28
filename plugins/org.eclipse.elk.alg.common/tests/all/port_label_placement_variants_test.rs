use crate::elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::{
    SIDES_EAST_WEST, SIDES_NORTH_SOUTH, SIDES_SOUTH_WEST,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, ElkUtil};
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkLabelRef, ElkNodeRef, ElkPortRef};
use std::collections::VecDeque;
use std::path::PathBuf;

const EPSILON: f64 = 1e-6;
const LABEL_CHAR_WIDTH: f64 = 6.0;
const LABEL_HEIGHT: f64 = 10.0;

fn run_layout(graph: &ElkNodeRef) {
    let mut provider = LayeredLayoutProvider::new();
    provider.layout(graph, &mut BasicProgressMonitor::new());
}

fn apply_default_node_port_configuration(graph: &ElkNodeRef) {
    let children: Vec<ElkNodeRef> = graph.borrow_mut().children().iter().cloned().collect();
    let mut queue: VecDeque<ElkNodeRef> = VecDeque::from(children);
    while let Some(node) = queue.pop_front() {
        let (children, ports): (Vec<ElkNodeRef>, Vec<ElkPortRef>) = {
            let mut node_mut = node.borrow_mut();
            let children = node_mut.children().iter().cloned().collect();
            let ports = node_mut.ports().iter().cloned().collect();
            (children, ports)
        };

        ElkUtil::configure_with_default_values(&node);
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

fn collect_ports(root: &ElkNodeRef) -> Vec<ElkPortRef> {
    let mut queue = vec![root.clone()];
    let mut ports = Vec::new();
    while let Some(node) = queue.pop() {
        let (children, node_ports): (Vec<ElkNodeRef>, Vec<ElkPortRef>) = {
            let mut node_mut = node.borrow_mut();
            let children = node_mut.children().iter().cloned().collect();
            let ports = node_mut.ports().iter().cloned().collect();
            (children, ports)
        };
        queue.extend(children);
        ports.extend(node_ports);
    }
    ports
}

fn apply_port_side(graph: &ElkNodeRef, side: PortSide) {
    for port in collect_ports(graph) {
        port.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_SIDE, Some(side));
    }
}

fn load_variants_graph(side_override: Option<PortSide>) -> ElkNodeRef {
    initialize_plain_java_layout();

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../external/elk-models/tests/core/label_placement/port_labels/variants.elkt");
    let path = path.to_string_lossy().into_owned();
    let graph = load_layered_graph_from_elkt(&path)
        .unwrap_or_else(|err| panic!("port label variants resource should load: {err}"));

    if let Some(side) = side_override {
        apply_port_side(&graph, side);
    }

    apply_default_node_port_configuration(&graph);
    ensure_label_sizes(&graph);
    run_layout(&graph);
    graph
}

fn port_index(port: &ElkPortRef) -> i32 {
    port.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(LayeredOptions::PORT_INDEX)
        .unwrap_or(0)
}

fn port_side(port: &ElkPortRef) -> PortSide {
    port.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(CoreOptions::PORT_SIDE)
        .unwrap_or(PortSide::Undefined)
}

fn ports_of_node(graph: &ElkNodeRef, node_id: &str) -> Vec<ElkPortRef> {
    let node = find_node_by_identifier(graph, node_id)
        .unwrap_or_else(|| panic!("node {node_id} should exist"));
    let mut ports: Vec<_> = node.borrow_mut().ports().iter().cloned().collect();
    ports.sort_by(|left, right| {
        let left_side = port_side(left);
        let left_index = port_index(left);
        let right_index = port_index(right);
        if SIDES_SOUTH_WEST.contains(&left_side) {
            left_index.cmp(&right_index)
        } else {
            right_index.cmp(&left_index)
        }
    });
    ports
}

fn label_for_port(port: &ElkPortRef) -> ElkLabelRef {
    let label = {
        let mut port_mut = port.borrow_mut();
        port_mut
            .connectable()
            .shape()
            .graph_element()
            .labels()
            .get(0)
    };
    label.expect("port should have one label")
}

fn label_axis_position(port: &ElkPortRef) -> f64 {
    let side = port_side(port);
    let label = label_for_port(port);
    if SIDES_EAST_WEST.contains(&side) {
        label.borrow_mut().shape().y()
    } else if SIDES_NORTH_SOUTH.contains(&side) {
        label.borrow_mut().shape().x()
    } else {
        panic!("port side should be resolved");
    }
}

fn assert_below_or_right(port: ElkPortRef, context: &str) {
    let position = label_axis_position(&port);
    if std::env::var("DEBUG_PORT_LABEL_VARIANTS").is_ok() {
        let label_shape = {
            let label_ref = label_for_port(&port);
            let mut label_mut = label_ref.borrow_mut();
            let label_shape = label_mut.shape();
            (
                label_shape.x(),
                label_shape.y(),
                label_shape.width(),
                label_shape.height(),
            )
        };
        let (label_x, label_y, label_w, label_h) = label_shape;

        let port_shape = {
            let mut port_mut = port.borrow_mut();
            let port_shape = port_mut.connectable().shape();
            (
                port_shape.x(),
                port_shape.y(),
                port_shape.width(),
                port_shape.height(),
                port_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(|id| id.to_string()),
            )
        };
        let (port_x, port_y, port_w, port_h, port_id) = port_shape;

        let parent_id = {
            let port_mut = port.borrow_mut();
            port_mut.parent().and_then(|parent| {
                parent
                    .borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(|id| id.to_string())
            })
        };

        let side = port_side(&port);
        println!(
            "[DEBUG][{context}] parent={:?} port={:?} side={side:?} label_pos=({}, {}) size=({label_w},{label_h}) port_pos=({port_x},{port_y}) size=({port_w},{port_h}) axis={position}",
            parent_id,
            port_id,
            label_x,
            label_y
        );
    }
    assert!(
        position > 0.0,
        "{context}: expected label position > 0, got {position}"
    );
}

fn assert_above_or_left(port: ElkPortRef, context: &str) {
    let position = label_axis_position(&port);
    if std::env::var("DEBUG_PORT_LABEL_VARIANTS").is_ok() {
        let label_shape = {
            let label_ref = label_for_port(&port);
            let mut label_mut = label_ref.borrow_mut();
            let label_shape = label_mut.shape();
            (
                label_shape.x(),
                label_shape.y(),
                label_shape.width(),
                label_shape.height(),
            )
        };
        let (label_x, label_y, label_w, label_h) = label_shape;

        let port_shape = {
            let mut port_mut = port.borrow_mut();
            let port_shape = port_mut.connectable().shape();
            (
                port_shape.x(),
                port_shape.y(),
                port_shape.width(),
                port_shape.height(),
                port_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(|id| id.to_string()),
            )
        };
        let (port_x, port_y, port_w, port_h, port_id) = port_shape;

        let parent_id = {
            let port_mut = port.borrow_mut();
            port_mut.parent().and_then(|parent| {
                parent
                    .borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(|id| id.to_string())
            })
        };

        let side = port_side(&port);
        println!(
            "[DEBUG][{context}] parent={:?} port={:?} side={side:?} label_pos=({}, {}) size=({label_w},{label_h}) port_pos=({port_x},{port_y}) size=({port_w},{port_h}) axis={position}",
            parent_id,
            port_id,
            label_x,
            label_y
        );
    }
    assert!(
        position < 0.0,
        "{context}: expected label position < 0, got {position}"
    );
}

fn assert_centered(port: ElkPortRef, context: &str) {
    let side = port_side(&port);
    let label = label_for_port(&port);
    if SIDES_EAST_WEST.contains(&side) {
        let (port_center, label_center) = {
            let mut port_mut = port.borrow_mut();
            let mut label_mut = label.borrow_mut();
            let port_center = port_mut.connectable().shape().height() / 2.0;
            let label_shape = label_mut.shape();
            let label_center = label_shape.y() + label_shape.height() / 2.0;
            (port_center, label_center)
        };
        assert!(
            (label_center - port_center).abs() <= EPSILON,
            "{context}: expected centered label y, port_center={port_center}, label_center={label_center}"
        );
    } else if SIDES_NORTH_SOUTH.contains(&side) {
        let (port_center, label_center) = {
            let mut port_mut = port.borrow_mut();
            let mut label_mut = label.borrow_mut();
            let port_center = port_mut.connectable().shape().width() / 2.0;
            let label_shape = label_mut.shape();
            let label_center = label_shape.x() + label_shape.width() / 2.0;
            (port_center, label_center)
        };
        assert!(
            (label_center - port_center).abs() <= EPSILON,
            "{context}: expected centered label x, port_center={port_center}, label_center={label_center}"
        );
    } else {
        panic!("port side should be resolved");
    }
}

fn for_each_configuration(mut test: impl FnMut(&ElkNodeRef, &str)) {
    let configs = [
        (None, "default"),
        (Some(PortSide::West), "west"),
        (Some(PortSide::East), "east"),
    ];
    for (side, label) in configs {
        let graph = load_variants_graph(side);
        test(&graph, label);
    }
}

#[test]
fn test_outside_two_default() {
    for_each_configuration(|graph, config| {
        let ports = ports_of_node(graph, "outside_two_default");
        assert_eq!(ports.len(), 2, "unexpected graph for {config}");
        assert_below_or_right(ports[0].clone(), config);
        assert_above_or_left(ports[1].clone(), config);
    });
}

#[test]
fn test_outside_two_same_side() {
    for_each_configuration(|graph, config| {
        let ports = ports_of_node(graph, "outside_two_same_side");
        assert_eq!(ports.len(), 2, "unexpected graph for {config}");
        assert_below_or_right(ports[0].clone(), config);
        assert_below_or_right(ports[1].clone(), config);
    });
}

#[test]
fn test_outside_two_next_to_port() {
    for_each_configuration(|graph, config| {
        let ports = ports_of_node(graph, "outside_two_next_to_port");
        assert_eq!(ports.len(), 2, "unexpected graph for {config}");
        assert_centered(ports[0].clone(), config);
        assert_centered(ports[1].clone(), config);
    });
}

#[test]
fn test_outside_three_default() {
    for_each_configuration(|graph, config| {
        let ports = ports_of_node(graph, "outside_three_default");
        assert_eq!(ports.len(), 3, "unexpected graph for {config}");
        assert_below_or_right(ports[0].clone(), config);
        assert_below_or_right(ports[1].clone(), config);
        assert_below_or_right(ports[2].clone(), config);
    });
}

#[test]
fn test_outside_three_space_efficient() {
    for_each_configuration(|graph, config| {
        let ports = ports_of_node(graph, "outside_three_space_efficient");
        assert_eq!(ports.len(), 3, "unexpected graph for {config}");
        assert_below_or_right(ports[0].clone(), config);
        assert_below_or_right(ports[1].clone(), config);
        assert_above_or_left(ports[2].clone(), config);
    });
}

#[test]
fn test_inside_two_default() {
    for_each_configuration(|graph, config| {
        let ports = ports_of_node(graph, "inside_two_default");
        assert_eq!(ports.len(), 2, "unexpected graph for {config}");
        assert_centered(ports[0].clone(), config);
        assert_centered(ports[1].clone(), config);
    });
}

#[test]
fn test_inside_two_default_hierarchical() {
    for_each_configuration(|graph, config| {
        let ports = ports_of_node(graph, "inside_two_default_hierarchical");
        assert_eq!(ports.len(), 2, "unexpected graph for {config}");
        assert_below_or_right(ports[0].clone(), config);
        assert_below_or_right(ports[1].clone(), config);
    });
}

#[test]
fn test_inside_two_next_to_port_hierarchical() {
    for_each_configuration(|graph, config| {
        let ports = ports_of_node(graph, "inside_two_next_to_port_hierarchical");
        assert_eq!(ports.len(), 2, "unexpected graph for {config}");
        assert_centered(ports[0].clone(), config);
        assert_centered(ports[1].clone(), config);
    });
}

#[test]
fn test_inside_two_with_one_edge() {
    for_each_configuration(|graph, config| {
        let ports = ports_of_node(graph, "inside_two_with_one_edge");
        assert_eq!(ports.len(), 2, "unexpected graph for {config}");
        assert_below_or_right(ports[0].clone(), config);
        assert_below_or_right(ports[1].clone(), config);
    });
}

#[test]
fn test_inside_two_with_one_edge_next_to_port() {
    for_each_configuration(|graph, config| {
        let ports = ports_of_node(graph, "inside_two_with_one_edge_next_to_port");
        assert_eq!(ports.len(), 2, "unexpected graph for {config}");
        let port_with_edge = if has_incident_edge(&ports[0]) {
            ports[0].clone()
        } else {
            ports[1].clone()
        };
        let other = if has_incident_edge(&ports[0]) {
            ports[1].clone()
        } else {
            ports[0].clone()
        };
        assert_below_or_right(port_with_edge, config);
        assert_centered(other, config);
    });
}

#[test]
fn test_inside_three_with_one_edge() {
    for_each_configuration(|graph, config| {
        let ports = ports_of_node(graph, "inside_three_with_one_edge");
        assert_eq!(ports.len(), 3, "unexpected graph for {config}");
        assert_below_or_right(ports[0].clone(), config);
        assert_below_or_right(ports[1].clone(), config);
        assert_below_or_right(ports[2].clone(), config);
    });
}

#[test]
fn test_inside_three_with_one_edge_next_to_port() {
    for_each_configuration(|graph, config| {
        let ports = ports_of_node(graph, "inside_three_with_one_edge_next_to_port");
        assert_eq!(ports.len(), 3, "unexpected graph for {config}");
        assert_centered(ports[0].clone(), config);
        assert_below_or_right(ports[1].clone(), config);
        assert_centered(ports[2].clone(), config);
    });
}

fn has_incident_edge(port: &ElkPortRef) -> bool {
    let mut port_mut = port.borrow_mut();
    !port_mut.connectable().incoming_edges().is_empty()
        || !port_mut.connectable().outgoing_edges().is_empty()
}
