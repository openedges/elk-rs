use std::rc::Rc;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::{
    NodeMicroLayout,
    nodespacing::{NodeDimensionCalculation, NodeMarginCalculator},
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMargin, ElkPadding};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, EdgeLabelPlacement, PortConstraints, PortLabelPlacement, PortSide,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::ElkGraphAdapters;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkGraphElementRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

fn approx_eq(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

fn set_node_geometry(node: &ElkNodeRef, x: f64, y: f64, width: f64, height: f64) {
    node.borrow_mut().connectable().shape().set_location(x, y);
    node.borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(width, height);
}

fn set_port_geometry(port: &ElkPortRef, x: f64, y: f64, width: f64, height: f64) {
    port.borrow_mut().connectable().shape().set_location(x, y);
    port.borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(width, height);
}

fn set_label_geometry(label: &ElkLabelRef, x: f64, y: f64, width: f64, height: f64) {
    label.borrow_mut().shape().set_location(x, y);
    label.borrow_mut().shape().set_dimensions(width, height);
}

fn node_margin(node: &ElkNodeRef) -> ElkMargin {
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(CoreOptions::MARGINS)
        .unwrap_or_default()
}

fn create_single_node_graph() -> (ElkNodeRef, ElkNodeRef) {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_geometry(&node, 10.0, 20.0, 40.0, 30.0);
    (graph, node)
}

fn outside_north_label_x_positions(
    placements: &[PortLabelPlacement],
    port_count: usize,
) -> Vec<f64> {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedOrder));
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_LABELS_PLACEMENT, Some(EnumSet::of(placements)));

    let mut ports = Vec::new();
    for index in 0..port_count {
        let port = ElkGraphUtil::create_port(Some(node.clone()));
        set_port_geometry(&port, (index as f64) * 12.0, 0.0, 8.0, 4.0);
        port.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_SIDE, Some(PortSide::North));
        port.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_INDEX, Some(index as i32));
        let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Port(port.clone())));
        set_label_geometry(&label, 0.0, 0.0, 6.0, 2.0);
        ports.push(port);
    }

    NodeMicroLayout::for_graph(graph).execute();

    ports
        .iter()
        .map(|port| {
            let label = {
                let mut port_mut = port.borrow_mut();
                let labels: Vec<_> = port_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect();
                labels.first().expect("port label").clone()
            };
            let mut label_mut = label.borrow_mut();
            label_mut.shape().x()
        })
        .collect()
}

fn constrained_outside_north_label_rectangles() -> Vec<(f64, f64, f64, f64)> {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedOrder));
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(
            CoreOptions::PORT_LABELS_PLACEMENT,
            Some(EnumSet::of(&[
                PortLabelPlacement::Outside,
                PortLabelPlacement::AlwaysSameSide,
            ])),
        );
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::NODE_SIZE_CONSTRAINTS, Some(EnumSet::none_of()));

    let mut ports = Vec::new();
    for index in 0..3 {
        let port = ElkGraphUtil::create_port(Some(node.clone()));
        set_port_geometry(&port, (index as f64) * 3.0, 0.0, 8.0, 4.0);
        port.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_SIDE, Some(PortSide::North));
        port.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_INDEX, Some(index));
        let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Port(port.clone())));
        set_label_geometry(&label, 0.0, 0.0, 24.0, 6.0);
        ports.push(port);
    }

    NodeMicroLayout::for_graph(graph).execute();

    let mut rects = ports
        .iter()
        .map(|port| {
            let (port_x, port_y, label_ref) = {
                let mut port_mut = port.borrow_mut();
                let shape = port_mut.connectable().shape();
                let labels: Vec<_> = shape
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect();
                (
                    shape.x(),
                    shape.y(),
                    labels.first().expect("port label should exist").clone(),
                )
            };
            let mut label_mut = label_ref.borrow_mut();
            let label_shape = label_mut.shape();
            (
                port_x + label_shape.x(),
                port_y + label_shape.y(),
                label_shape.width(),
                label_shape.height(),
            )
        })
        .collect::<Vec<_>>();
    rects.sort_by(|left, right| left.0.partial_cmp(&right.0).unwrap_or(std::cmp::Ordering::Equal));
    rects
}

fn constrained_inside_horizontal_label_rectangles(port_side: PortSide) -> Vec<(f64, f64, f64, f64)> {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_geometry(&node, 0.0, 0.0, 40.0, 40.0);
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedOrder));
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(
            CoreOptions::PORT_LABELS_PLACEMENT,
            Some(EnumSet::of(&[
                PortLabelPlacement::Inside,
                PortLabelPlacement::AlwaysSameSide,
            ])),
        );
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::NODE_SIZE_CONSTRAINTS, Some(EnumSet::none_of()));

    let mut ports = Vec::new();
    for index in 0..3 {
        let port = ElkGraphUtil::create_port(Some(node.clone()));
        let y = match port_side {
            PortSide::North => 0.0,
            PortSide::South => 36.0,
            _ => panic!("helper only supports north/south"),
        };
        set_port_geometry(&port, (index as f64) * 2.0, y, 8.0, 4.0);
        port.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_SIDE, Some(port_side));
        port.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_INDEX, Some(index));
        let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Port(port.clone())));
        set_label_geometry(&label, 0.0, 0.0, 34.0, 6.0);
        ports.push(port);
    }

    NodeMicroLayout::for_graph(graph).execute();

    ports
        .iter()
        .map(|port| {
            let (port_x, port_y, label_ref) = {
                let mut port_mut = port.borrow_mut();
                let shape = port_mut.connectable().shape();
                let labels: Vec<_> = shape
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect();
                (
                    shape.x(),
                    shape.y(),
                    labels.first().expect("port label should exist").clone(),
                )
            };
            let mut label_mut = label_ref.borrow_mut();
            let label_shape = label_mut.shape();
            (
                port_x + label_shape.x(),
                port_y + label_shape.y(),
                label_shape.width(),
                label_shape.height(),
            )
        })
        .collect()
}

fn constrained_inside_custom_label_rectangles(
    port_side: PortSide,
    node_width: f64,
    port_x_positions: &[f64],
    label_width: f64,
) -> Vec<(f64, f64, f64, f64)> {
    constrained_inside_custom_label_rectangles_with_options(
        port_side,
        node_width,
        port_x_positions,
        label_width,
        None,
        None,
        None,
    )
}

fn constrained_inside_custom_label_rectangles_with_options(
    port_side: PortSide,
    node_width: f64,
    port_x_positions: &[f64],
    label_width: f64,
    node_padding: Option<ElkPadding>,
    label_node_spacing: Option<f64>,
    node_labels_padding: Option<ElkPadding>,
) -> Vec<(f64, f64, f64, f64)> {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_geometry(&node, 0.0, 0.0, node_width, 40.0);
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedOrder));
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(
            CoreOptions::PORT_LABELS_PLACEMENT,
            Some(EnumSet::of(&[
                PortLabelPlacement::Inside,
                PortLabelPlacement::AlwaysSameSide,
            ])),
        );
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::NODE_SIZE_CONSTRAINTS, Some(EnumSet::none_of()));
    if let Some(padding) = node_padding {
        node.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PADDING, Some(padding));
    }
    if let Some(spacing) = label_node_spacing {
        node.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::SPACING_LABEL_NODE, Some(spacing));
    }
    if let Some(node_labels_padding) = node_labels_padding {
        node.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::NODE_LABELS_PADDING, Some(node_labels_padding));
    }

    let mut ports = Vec::new();
    for (index, port_x) in port_x_positions.iter().enumerate() {
        let port = ElkGraphUtil::create_port(Some(node.clone()));
        let y = match port_side {
            PortSide::North => 0.0,
            PortSide::South => 36.0,
            _ => panic!("helper only supports north/south"),
        };
        set_port_geometry(&port, *port_x, y, 8.0, 4.0);
        port.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_SIDE, Some(port_side));
        port.borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_INDEX, Some(index as i32));
        let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Port(port.clone())));
        set_label_geometry(&label, 0.0, 0.0, label_width, 6.0);
        ports.push(port);
    }

    NodeMicroLayout::for_graph(graph).execute();

    ports
        .iter()
        .map(|port| {
            let (port_x, port_y, label_ref) = {
                let mut port_mut = port.borrow_mut();
                let shape = port_mut.connectable().shape();
                let labels: Vec<_> = shape
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect();
                (
                    shape.x(),
                    shape.y(),
                    labels.first().expect("port label should exist").clone(),
                )
            };
            let mut label_mut = label_ref.borrow_mut();
            let label_shape = label_mut.shape();
            (
                port_x + label_shape.x(),
                port_y + label_shape.y(),
                label_shape.width(),
                label_shape.height(),
            )
        })
        .collect()
}

fn create_port_tail_label_setup() -> (ElkNodeRef, ElkNodeRef, ElkPortRef, ElkEdgeRef, ElkLabelRef) {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    graph.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::SPACING_LABEL_NODE, Some(2.0));

    let source = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_geometry(&source, 0.0, 0.0, 20.0, 20.0);
    source
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(
            CoreOptions::PORT_LABELS_PLACEMENT,
            Some(EnumSet::of(&[PortLabelPlacement::Outside])),
        );

    let port = ElkGraphUtil::create_port(Some(source.clone()));
    set_port_geometry(&port, 20.0, 8.0, 2.0, 4.0);
    port.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_SIDE, Some(PortSide::East));

    let port_label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Port(port.clone())));
    set_label_geometry(&port_label, 0.0, 0.0, 10.0, 4.0);

    let target = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_geometry(&target, 80.0, 0.0, 20.0, 20.0);
    let edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(port.clone()),
        ElkConnectableShapeRef::Node(target),
    );
    let edge_label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Edge(edge.clone())));
    set_label_geometry(&edge_label, 0.0, 0.0, 6.0, 3.0);
    edge_label
        .borrow_mut()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(
            CoreOptions::EDGE_LABELS_PLACEMENT,
            Some(EdgeLabelPlacement::Tail),
        );

    (graph, source, port, edge, edge_label)
}

#[test]
fn node_margin_calculator_includes_node_labels() {
    let (graph, node) = create_single_node_graph();
    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Node(node.clone())));
    set_label_geometry(&label, -6.0, 5.0, 4.0, 8.0);

    let adapter = ElkGraphAdapters::adapt(graph);
    let mut calculator = NodeMarginCalculator::new(&adapter);
    calculator.process();

    let margin = node_margin(&node);
    approx_eq(margin.left, 6.0);
}

#[test]
fn node_margin_calculator_excludes_node_labels_when_configured() {
    let (graph, node) = create_single_node_graph();
    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Node(node.clone())));
    set_label_geometry(&label, -6.0, 5.0, 4.0, 8.0);

    let adapter = ElkGraphAdapters::adapt(graph);
    let mut calculator = NodeMarginCalculator::new(&adapter);
    calculator.exclude_labels().process();

    let margin = node_margin(&node);
    approx_eq(margin.left, 0.0);
}

#[test]
fn node_margin_calculator_includes_ports() {
    let (graph, node) = create_single_node_graph();
    let port = ElkGraphUtil::create_port(Some(node.clone()));
    set_port_geometry(&port, 38.0, 8.0, 6.0, 10.0);

    let adapter = ElkGraphAdapters::adapt(graph);
    let mut calculator = NodeMarginCalculator::new(&adapter);
    calculator.process();

    let margin = node_margin(&node);
    approx_eq(margin.right, 4.0);
}

#[test]
fn node_margin_calculator_excludes_ports_when_configured() {
    let (graph, node) = create_single_node_graph();
    let port = ElkGraphUtil::create_port(Some(node.clone()));
    set_port_geometry(&port, 38.0, 8.0, 6.0, 10.0);

    let adapter = ElkGraphAdapters::adapt(graph);
    let mut calculator = NodeMarginCalculator::new(&adapter);
    calculator.exclude_ports().process();

    let margin = node_margin(&node);
    approx_eq(margin.right, 0.0);
}

#[test]
fn node_margin_calculator_includes_port_labels() {
    let (graph, node) = create_single_node_graph();
    let port = ElkGraphUtil::create_port(Some(node.clone()));
    set_port_geometry(&port, 5.0, 10.0, 4.0, 4.0);
    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Port(port)));
    set_label_geometry(&label, 36.0, 0.0, 10.0, 4.0);

    let adapter = ElkGraphAdapters::adapt(graph);
    let mut calculator = NodeMarginCalculator::new(&adapter);
    calculator.process();

    let margin = node_margin(&node);
    approx_eq(margin.right, 11.0);
}

#[test]
fn node_margin_calculator_excludes_port_labels_when_configured() {
    let (graph, node) = create_single_node_graph();
    let port = ElkGraphUtil::create_port(Some(node.clone()));
    set_port_geometry(&port, 5.0, 10.0, 4.0, 4.0);
    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Port(port)));
    set_label_geometry(&label, 36.0, 0.0, 10.0, 4.0);

    let adapter = ElkGraphAdapters::adapt(graph);
    let mut calculator = NodeMarginCalculator::new(&adapter);
    calculator.exclude_port_labels().process();

    let margin = node_margin(&node);
    approx_eq(margin.right, 0.0);
}

#[test]
fn node_margin_calculator_includes_tail_label_for_east_port_and_outside_port_labels() {
    let (graph, source, ..) = create_port_tail_label_setup();

    let adapter = ElkGraphAdapters::adapt(graph);
    let mut calculator = NodeMarginCalculator::new(&adapter);
    calculator.process();

    let margin = node_margin(&source);
    approx_eq(margin.right, 22.0);
}

#[test]
fn node_margin_calculator_ignores_edge_end_labels_when_disabled() {
    let (graph, source, ..) = create_port_tail_label_setup();

    let adapter = ElkGraphAdapters::adapt(graph);
    let mut calculator = NodeMarginCalculator::new(&adapter);
    calculator.exclude_edge_head_tail_labels().process();

    let margin = node_margin(&source);
    approx_eq(margin.right, 10.0);
}

#[test]
fn node_margin_calculator_places_head_label_left_of_west_port() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    graph.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::SPACING_LABEL_NODE, Some(2.0));

    let source = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_geometry(&source, 0.0, 0.0, 20.0, 20.0);

    let target = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_geometry(&target, 50.0, 0.0, 20.0, 20.0);
    let target_port = ElkGraphUtil::create_port(Some(target.clone()));
    set_port_geometry(&target_port, -2.0, 8.0, 2.0, 4.0);
    target_port
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_SIDE, Some(PortSide::West));

    let edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(source),
        ElkConnectableShapeRef::Port(target_port),
    );
    let edge_label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Edge(edge)));
    set_label_geometry(&edge_label, 0.0, 0.0, 6.0, 3.0);
    edge_label
        .borrow_mut()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(
            CoreOptions::EDGE_LABELS_PLACEMENT,
            Some(EdgeLabelPlacement::Head),
        );

    let adapter = ElkGraphAdapters::adapt(graph);
    let mut calculator = NodeMarginCalculator::new(&adapter);
    calculator.process();

    let margin = node_margin(&target);
    approx_eq(margin.left, 12.0);
}

#[test]
fn node_dimension_calculation_sorts_ports_by_side_and_index() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedOrder));

    let p1 = ElkGraphUtil::create_port(Some(node.clone()));
    let p2 = ElkGraphUtil::create_port(Some(node.clone()));
    let p3 = ElkGraphUtil::create_port(Some(node.clone()));

    p1.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_SIDE, Some(PortSide::West));
    p1.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_INDEX, Some(2));

    p2.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_SIDE, Some(PortSide::East));
    p2.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_INDEX, Some(0));

    p3.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_SIDE, Some(PortSide::West));
    p3.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_INDEX, Some(1));

    let adapter = ElkGraphAdapters::adapt(graph);
    NodeDimensionCalculation::sort_port_lists(&adapter);

    let ports: Vec<_> = {
        let mut node_mut = node.borrow_mut();
        node_mut.ports().iter().cloned().collect()
    };
    assert!(Rc::ptr_eq(&ports[0], &p2));
    assert!(Rc::ptr_eq(&ports[1], &p3));
    assert!(Rc::ptr_eq(&ports[2], &p1));
}

#[test]
fn node_dimension_calculation_generic_dispatch_places_port_labels_for_elk_adapter() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(
            CoreOptions::PORT_LABELS_PLACEMENT,
            Some(EnumSet::of(&[PortLabelPlacement::Outside])),
        );

    let port = ElkGraphUtil::create_port(Some(node.clone()));
    set_port_geometry(&port, 0.0, 0.0, 8.0, 4.0);
    port.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_SIDE, Some(PortSide::North));

    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Port(port)));
    set_label_geometry(&label, 0.0, 0.0, 6.0, 2.0);

    let adapter = ElkGraphAdapters::adapt(graph);
    NodeDimensionCalculation::calculate_label_and_node_sizes(&adapter);

    let mut label_mut = label.borrow_mut();
    let shape = label_mut.shape();
    approx_eq(shape.x(), 9.0);
}

#[test]
fn node_micro_layout_executes_port_sort_and_margin_calculation() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_geometry(&node, 0.0, 0.0, 20.0, 20.0);
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedOrder));

    let p1 = ElkGraphUtil::create_port(Some(node.clone()));
    let p2 = ElkGraphUtil::create_port(Some(node.clone()));
    p1.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_SIDE, Some(PortSide::West));
    p1.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_INDEX, Some(2));
    p2.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_SIDE, Some(PortSide::West));
    p2.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::PORT_INDEX, Some(1));

    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Node(node.clone())));
    set_label_geometry(&label, -5.0, 0.0, 4.0, 4.0);

    NodeMicroLayout::for_graph(graph).execute();

    let margin = node_margin(&node);
    assert!(margin.left > 0.0);

    let ports: Vec<_> = {
        let mut node_mut = node.borrow_mut();
        node_mut.ports().iter().cloned().collect()
    };
    assert!(Rc::ptr_eq(&ports[0], &p2));
    assert!(Rc::ptr_eq(&ports[1], &p1));
}

#[test]
fn node_micro_layout_does_not_resize_node_when_label_size_calculation_is_noop() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_geometry(&node, 0.0, 0.0, 100.0, 100.0);
    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Node(node.clone())));
    set_label_geometry(&label, 0.0, 0.0, 30.0, 10.0);

    NodeMicroLayout::for_graph(graph).execute();

    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    approx_eq(shape.width(), 100.0);
    approx_eq(shape.height(), 100.0);
}

#[test]
fn node_micro_layout_does_not_move_existing_node_label_positions() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_geometry(&node, 0.0, 0.0, 80.0, 60.0);
    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Node(node)));
    set_label_geometry(&label, 7.0, 9.0, 15.0, 8.0);

    NodeMicroLayout::for_graph(graph).execute();

    let mut label_mut = label.borrow_mut();
    let label_shape = label_mut.shape();
    approx_eq(label_shape.x(), 7.0);
    approx_eq(label_shape.y(), 9.0);
}

fn should_label_be_placed_next_to_port(
    port: &ElkPortRef,
    parent: &ElkNodeRef,
    inside_label_placement: bool,
) -> bool {
    let (incoming_edges, outgoing_edges) = {
        let mut port_mut = port.borrow_mut();
        let incoming_edges = port_mut
            .connectable()
            .incoming_edges()
            .iter()
            .collect::<Vec<_>>();
        let outgoing_edges = port_mut
            .connectable()
            .outgoing_edges()
            .iter()
            .collect::<Vec<_>>();
        (incoming_edges, outgoing_edges)
    };

    if incoming_edges.is_empty() && outgoing_edges.is_empty() {
        return true;
    }

    let mut edges_to_insides = false;
    let mut edges_to_somewhere_else = false;

    for out_edge in outgoing_edges {
        let target = {
            let edge = out_edge.borrow();
            edge.targets_ro().get(0)
        };
        if let Some(target_shape) = target {
            if let Some(target_node) = ElkGraphUtil::connectable_shape_to_node(&target_shape) {
                let inside_edge = ElkGraphUtil::is_descendant(&target_node, parent);
                edges_to_insides |= inside_edge;
                edges_to_somewhere_else |= !inside_edge;
            }
        }
    }

    for in_edge in incoming_edges {
        let source = {
            let edge = in_edge.borrow();
            edge.sources_ro().get(0)
        };
        if let Some(source_shape) = source {
            if let Some(source_node) = ElkGraphUtil::connectable_shape_to_node(&source_shape) {
                let inside_edge = ElkGraphUtil::is_descendant(&source_node, parent);
                edges_to_insides |= inside_edge;
                edges_to_somewhere_else |= !inside_edge;
            }
        }
    }

    (inside_label_placement && !edges_to_insides) || (!inside_label_placement && !edges_to_somewhere_else)
}

fn create_next_to_port_rule_graph() -> (ElkNodeRef, ElkNodeRef, ElkNodeRef, ElkNodeRef, ElkPortRef) {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let parent = ElkGraphUtil::create_node(Some(graph.clone()));
    let child = ElkGraphUtil::create_node(Some(parent.clone()));
    let outside = ElkGraphUtil::create_node(Some(graph.clone()));
    let port = ElkGraphUtil::create_port(Some(parent.clone()));
    (graph, parent, child, outside, port)
}

#[test]
fn next_to_port_rule_without_incident_edges() {
    let (_graph, parent, _, _, port) = create_next_to_port_rule_graph();
    assert!(should_label_be_placed_next_to_port(&port, &parent, true));
    assert!(should_label_be_placed_next_to_port(&port, &parent, false));
}

#[test]
fn next_to_port_rule_inside_placement_rejects_inside_edges() {
    let (_graph, parent, child, _, port) = create_next_to_port_rule_graph();
    ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Port(port.clone()),
        ElkConnectableShapeRef::Node(child),
    );
    assert!(!should_label_be_placed_next_to_port(&port, &parent, true));
}

#[test]
fn next_to_port_rule_inside_placement_accepts_only_external_edges() {
    let (_graph, parent, _, outside, port) = create_next_to_port_rule_graph();
    ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(outside),
        ElkConnectableShapeRef::Port(port.clone()),
    );
    assert!(should_label_be_placed_next_to_port(&port, &parent, true));
}

#[test]
fn next_to_port_rule_outside_placement_rejects_external_edges() {
    let (_graph, parent, _, outside, port) = create_next_to_port_rule_graph();
    ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(outside),
        ElkConnectableShapeRef::Port(port.clone()),
    );
    assert!(!should_label_be_placed_next_to_port(&port, &parent, false));
}

#[test]
fn outside_two_ports_places_first_label_on_other_side() {
    let positions = outside_north_label_x_positions(&[PortLabelPlacement::Outside], 2);
    approx_eq(positions[0], -7.0);
    approx_eq(positions[1], 9.0);
}

#[test]
fn outside_two_ports_with_always_same_side_disables_first_special_case() {
    let positions = outside_north_label_x_positions(
        &[PortLabelPlacement::Outside, PortLabelPlacement::AlwaysSameSide],
        2,
    );
    approx_eq(positions[0], 9.0);
    approx_eq(positions[1], 9.0);
}

#[test]
fn outside_three_ports_with_space_efficient_places_first_label_on_other_side() {
    let positions = outside_north_label_x_positions(
        &[PortLabelPlacement::Outside, PortLabelPlacement::SpaceEfficient],
        3,
    );
    approx_eq(positions[0], -7.0);
    approx_eq(positions[1], 9.0);
    approx_eq(positions[2], 9.0);
}

#[test]
fn constrained_outside_north_port_labels_stack_without_overlap() {
    let rects = constrained_outside_north_label_rectangles();
    assert_eq!(rects.len(), 3);

    for left_index in 0..rects.len() {
        for right_index in (left_index + 1)..rects.len() {
            let (lx, ly, lw, lh) = rects[left_index];
            let (rx, ry, rw, rh) = rects[right_index];
            let horizontal_overlap = lx < rx + rw && lx + lw > rx;
            let vertical_overlap = ly < ry + rh && ly + lh > ry;
            assert!(
                !(horizontal_overlap && vertical_overlap),
                "constrained outside placement should remove overlaps, left={:?}, right={:?}",
                rects[left_index],
                rects[right_index]
            );
        }
    }

    let has_stacked_row = rects.iter().any(|(_, y, _, _)| (*y - rects[0].1).abs() > 1e-9);
    assert!(
        has_stacked_row,
        "constrained placement should stack at least one label in a different y row, rects={rects:?}"
    );
}

#[test]
fn constrained_inside_north_port_labels_are_clamped_and_stacked() {
    let rects = constrained_inside_horizontal_label_rectangles(PortSide::North);
    assert_eq!(rects.len(), 3);

    for (x, y, w, h) in &rects {
        assert!(*x >= -1e-9, "inside north label should stay inside left bound, rect={rects:?}");
        assert!(
            *x + *w <= 40.0 + 1e-9,
            "inside north label should stay inside right bound, rect={rects:?}"
        );
        assert!(
            *y >= 5.0 - 1e-9,
            "inside north constrained placement should start below ports (port-bottom+gap), rect={rects:?}"
        );
        assert!(*h > 0.0);
    }

    for left_index in 0..rects.len() {
        for right_index in (left_index + 1)..rects.len() {
            let (lx, ly, lw, lh) = rects[left_index];
            let (rx, ry, rw, rh) = rects[right_index];
            let horizontal_overlap = lx < rx + rw && lx + lw > rx;
            let vertical_overlap = ly < ry + rh && ly + lh > ry;
            assert!(
                !(horizontal_overlap && vertical_overlap),
                "inside north constrained placement should remove overlaps, left={:?}, right={:?}",
                rects[left_index],
                rects[right_index]
            );
        }
    }
}

#[test]
fn constrained_inside_south_port_labels_are_clamped_and_stacked() {
    let rects = constrained_inside_horizontal_label_rectangles(PortSide::South);
    assert_eq!(rects.len(), 3);

    for (x, y, w, h) in &rects {
        assert!(*x >= -1e-9, "inside south label should stay inside left bound, rect={rects:?}");
        assert!(
            *x + *w <= 40.0 + 1e-9,
            "inside south label should stay inside right bound, rect={rects:?}"
        );
        assert!(
            *y + *h <= 35.0 + 1e-9,
            "inside south constrained placement should start above ports (port-top-gap), rect={rects:?}"
        );
    }

    for left_index in 0..rects.len() {
        for right_index in (left_index + 1)..rects.len() {
            let (lx, ly, lw, lh) = rects[left_index];
            let (rx, ry, rw, rh) = rects[right_index];
            let horizontal_overlap = lx < rx + rw && lx + lw > rx;
            let vertical_overlap = ly < ry + rh && ly + lh > ry;
            assert!(
                !(horizontal_overlap && vertical_overlap),
                "inside south constrained placement should remove overlaps, left={:?}, right={:?}",
                rects[left_index],
                rects[right_index]
            );
        }
    }
}

#[test]
fn constrained_inside_north_labels_center_then_clamp_to_port_extent() {
    let rects =
        constrained_inside_custom_label_rectangles(PortSide::North, 40.0, &[0.0, 35.0], 34.0);
    assert_eq!(rects.len(), 2);

    // left-most port: centered x would be negative, clamp to left boundary
    approx_eq(rects[0].0, 0.0);
    // right-most port: centered x would exceed right boundary, clamp using actualMaxX
    approx_eq(rects[1].0, 9.0);
}

#[test]
fn constrained_inside_south_labels_center_then_clamp_to_port_extent() {
    let rects =
        constrained_inside_custom_label_rectangles(PortSide::South, 40.0, &[0.0, 35.0], 34.0);
    assert_eq!(rects.len(), 2);

    approx_eq(rects[0].0, 0.0);
    approx_eq(rects[1].0, 9.0);
}

#[test]
fn constrained_inside_north_labels_respect_explicit_padding_and_spacing_bounds() {
    let rects = constrained_inside_custom_label_rectangles_with_options(
        PortSide::North,
        60.0,
        &[10.0, 45.0],
        30.0,
        Some(ElkPadding::with_values(0.0, 11.0, 0.0, 7.0)),
        Some(9.0),
        None,
    );
    assert_eq!(rects.len(), 2);

    approx_eq(rects[0].0, 9.0);
    approx_eq(rects[1].0, 23.0);
}

#[test]
fn constrained_inside_south_labels_respect_explicit_padding_and_spacing_bounds() {
    let rects = constrained_inside_custom_label_rectangles_with_options(
        PortSide::South,
        60.0,
        &[10.0, 45.0],
        30.0,
        Some(ElkPadding::with_values(0.0, 11.0, 0.0, 7.0)),
        Some(9.0),
        None,
    );
    assert_eq!(rects.len(), 2);

    approx_eq(rects[0].0, 9.0);
    approx_eq(rects[1].0, 23.0);
}

#[test]
fn constrained_inside_north_labels_respect_explicit_node_labels_padding_bounds() {
    let rects = constrained_inside_custom_label_rectangles_with_options(
        PortSide::North,
        60.0,
        &[10.0, 45.0],
        30.0,
        None,
        None,
        Some(ElkPadding::with_values(0.0, 13.0, 0.0, 8.0)),
    );
    assert_eq!(rects.len(), 2);

    approx_eq(rects[0].0, 8.0);
    approx_eq(rects[1].0, 23.0);
}

#[test]
fn constrained_inside_south_labels_respect_explicit_node_labels_padding_bounds() {
    let rects = constrained_inside_custom_label_rectangles_with_options(
        PortSide::South,
        60.0,
        &[10.0, 45.0],
        30.0,
        None,
        None,
        Some(ElkPadding::with_values(0.0, 13.0, 0.0, 8.0)),
    );
    assert_eq!(rects.len(), 2);

    approx_eq(rects[0].0, 8.0);
    approx_eq(rects[1].0, 23.0);
}
