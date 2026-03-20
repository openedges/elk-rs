/// Tests for the full CellSystem `process()` path in NodeLabelAndSizeCalculator.
///
/// These tests verify bugs that were discovered when switching from the simplified
/// `process_node()` path to the full 7-phase `process()` CellSystem pipeline:
///
/// 1. **Multi-label cell overwrite** (node_label_cell_creator): When two labels share
///    the same CellSystem location (e.g., both INSIDE V_TOP H_CENTER), the second label
///    must be added to the existing cell, not replace it.
///
/// 2. **Fixed port label insidePart** (node_label_and_size_utilities): When computing
///    how far a fixed port label extends into the node interior, the actual label position
///    must be used — not a zero default.
///
/// 3. **Edgeless root ports** (elk_graph_importer): Root-level ports with no edges must
///    still be treated as external ports when portConstraints >= FIXED_SIDE.
use std::sync::Once;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, NodeLabelPlacement, PortConstraints, PortLabelPlacement, PortSide, SizeConstraint,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, EnumSet};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkGraphElementRef, ElkNodeRef};

const EPS: f64 = 0.5;

fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
    });
}

fn run_layout(graph: &ElkNodeRef) {
    let mut provider = LayeredLayoutProvider::new();
    provider.layout(graph, &mut BasicProgressMonitor::new());
}

fn set_prop<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    prop: &org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<T>,
    value: T,
) {
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(prop, Some(value));
}

fn get_label_position(node: &ElkNodeRef, label_index: usize) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let labels = node_mut.connectable().shape().graph_element().labels();
    let label = labels.get(label_index).expect("label should exist");
    let mut label_mut = label.borrow_mut();
    (label_mut.shape().x(), label_mut.shape().y())
}

fn get_node_size(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    (
        node_mut.connectable().shape().width(),
        node_mut.connectable().shape().height(),
    )
}

// ============================================================
// Bug 1: Multi-label cell overwrite in NodeLabelCellCreator
// ============================================================

#[test]
fn cellsystem_multi_label_same_location_not_overwritten() {
    init();

    // Create a node with two labels at the same CellSystem location (INSIDE V_TOP H_CENTER).
    // Before the fix, the second label's cell replaced the first, leaving label[0] at (0,0).
    let graph = ElkGraphUtil::create_graph();
    set_prop(&graph, CoreOptions::ALGORITHM, LayeredOptions::ALGORITHM_ID.to_string());

    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    node.borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(200.0, 75.0);
    set_prop(
        &node,
        CoreOptions::NODE_SIZE_CONSTRAINTS,
        EnumSet::of(&[SizeConstraint::NodeLabels, SizeConstraint::MinimumSize]),
    );

    let placement = NodeLabelPlacement::inside_top_center();
    let label0 = ElkGraphUtil::create_label_with_text(
        "First Label",
        Some(ElkGraphElementRef::Node(node.clone())),
    );
    label0.borrow_mut().shape().set_dimensions(100.0, 15.0);
    label0
        .borrow_mut()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::NODE_LABELS_PLACEMENT, Some(placement.clone()));

    let label1 = ElkGraphUtil::create_label_with_text(
        "Second Label",
        Some(ElkGraphElementRef::Node(node.clone())),
    );
    label1.borrow_mut().shape().set_dimensions(117.0, 15.0);
    label1
        .borrow_mut()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::NODE_LABELS_PLACEMENT, Some(placement));

    run_layout(&graph);

    let (x0, y0) = get_label_position(&node, 0);
    let (x1, y1) = get_label_position(&node, 1);

    // Both labels should have non-zero positions (placed by CellSystem)
    assert!(
        x0 > EPS || y0 > EPS,
        "first label should be positioned (not at origin), got ({x0}, {y0})"
    );
    assert!(
        x1 > EPS || y1 > EPS,
        "second label should be positioned (not at origin), got ({x1}, {y1})"
    );

    // Second label should be below first (same column, stacked vertically)
    assert!(
        y1 > y0 + 10.0,
        "second label y={y1} should be below first label y={y0}"
    );
}

// ============================================================
// Bug 2: Fixed port label insidePart computed with zero position
// ============================================================

#[test]
fn cellsystem_fixed_port_label_inside_part_uses_real_position() {
    init();

    // Create a node with a NORTH port whose fixed label extends below (into the node).
    // Before the fix, compute_inside_part used (0,0) for the label position,
    // underestimating the space needed.
    let graph = ElkGraphUtil::create_graph();
    set_prop(&graph, CoreOptions::ALGORITHM, LayeredOptions::ALGORITHM_ID.to_string());

    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    node.borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(100.0, 50.0);
    set_prop(
        &node,
        CoreOptions::NODE_SIZE_CONSTRAINTS,
        EnumSet::of(&[
            SizeConstraint::Ports,
            SizeConstraint::NodeLabels,
            SizeConstraint::PortLabels,
        ]),
    );
    set_prop(&node, CoreOptions::PORT_CONSTRAINTS, PortConstraints::FixedSide);
    // Fixed (empty) port label placement
    set_prop(
        &node,
        CoreOptions::PORT_LABELS_PLACEMENT,
        EnumSet::<PortLabelPlacement>::none_of(),
    );

    let node_label = ElkGraphUtil::create_label_with_text(
        "MyNode",
        Some(ElkGraphElementRef::Node(node.clone())),
    );
    node_label.borrow_mut().shape().set_dimensions(50.0, 17.0);
    node_label
        .borrow_mut()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(
            CoreOptions::NODE_LABELS_PLACEMENT,
            Some(NodeLabelPlacement::inside_top_center()),
        );

    // North port with label extending below into node
    let port = ElkGraphUtil::create_port(Some(node.clone()));
    port.borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(20.0, 20.0);
    {
        let mut port_mut = port.borrow_mut();
        port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_SIDE, Some(PortSide::North));
        port_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::PORT_BORDER_OFFSET, Some(-8.0_f64));
    }

    let port_label = ElkGraphUtil::create_label_with_text(
        "NorthFixedLabel",
        Some(ElkGraphElementRef::Port(port.clone())),
    );
    port_label.borrow_mut().shape().set_location(-40.0, 21.0);
    port_label
        .borrow_mut()
        .shape()
        .set_dimensions(100.0, 17.0);

    // Add a target node and edge so layered algorithm has something to layout
    let target = ElkGraphUtil::create_node(Some(graph.clone()));
    target
        .borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(30.0, 20.0);

    run_layout(&graph);

    let (_, h) = get_node_size(&node);

    // The node must be tall enough to accommodate the port label extending into it.
    // Port label extends from y=(-8+21)=13 to y=(13+17)=30 inside the node,
    // plus node label and padding. Total should be well above 50.
    assert!(
        h > 60.0,
        "node height {h} should accommodate north port label extension (expected > 60)"
    );
}
// Edgeless root port tests: see root_external_ports_test.rs in org-eclipse-elk-graph-json
