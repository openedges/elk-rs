use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkPadding, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;

use crate::common::issue_support::{
    create_edge, create_graph, create_node, init_layered_options, node_bounds, run_layout,
    set_node_property,
};

const GRID: f64 = 10.0;

fn is_grid_multiple(value: f64, grid: f64) -> bool {
    let remainder = (value / grid).round() * grid - value;
    remainder.abs() < 1e-9
}

/// Create a grid-snap-enabled child node with SizeConstraints so the layout transferrer
/// writes snapped sizes back to the ElkGraph.
fn create_grid_node(parent: &ElkNodeRef, width: f64, height: f64) -> ElkNodeRef {
    let node = create_node(parent, width, height);
    set_node_property(
        &node,
        CoreOptions::NODE_SIZE_CONSTRAINTS,
        EnumSet::of(&[SizeConstraint::MinimumSize]),
    );
    node
}

fn layout_grid_graph(direction: Direction) -> ElkNodeRef {
    init_layered_options();
    let graph = create_graph();
    set_node_property(&graph, LayeredOptions::GRID_SNAP_GRID_SIZE, GRID);
    set_node_property(&graph, CoreOptions::DIRECTION, direction);
    // Padding must be a grid multiple for absolute coordinates to stay grid-aligned.
    set_node_property(&graph, CoreOptions::PADDING, ElkPadding::with_any(GRID));

    let n1 = create_grid_node(&graph, 37.0, 53.0);
    let n2 = create_grid_node(&graph, 41.0, 27.0);
    let n3 = create_grid_node(&graph, 23.0, 45.0);

    create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    create_edge(
        ElkConnectableShapeRef::Node(n2.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );

    run_layout(&graph);
    graph
}

fn assert_all_nodes_grid_aligned(graph: &ElkNodeRef, grid: f64) {
    let children: Vec<ElkNodeRef> = graph.borrow_mut().children().iter().cloned().collect();
    for child in &children {
        let (x, y, w, h) = node_bounds(child);
        assert!(
            is_grid_multiple(x, grid),
            "node x={x} is not a multiple of {grid}"
        );
        assert!(
            is_grid_multiple(y, grid),
            "node y={y} is not a multiple of {grid}"
        );
        assert!(
            is_grid_multiple(w, grid),
            "node width={w} is not a multiple of {grid}"
        );
        assert!(
            is_grid_multiple(h, grid),
            "node height={h} is not a multiple of {grid}"
        );
    }
}

#[test]
fn grid_snap_full_pipeline_right() {
    let graph = layout_grid_graph(Direction::Right);
    assert_all_nodes_grid_aligned(&graph, GRID);
}

#[test]
fn grid_snap_full_pipeline_left() {
    let graph = layout_grid_graph(Direction::Left);
    assert_all_nodes_grid_aligned(&graph, GRID);
}

#[test]
fn grid_snap_full_pipeline_down() {
    let graph = layout_grid_graph(Direction::Down);
    assert_all_nodes_grid_aligned(&graph, GRID);
}

#[test]
fn grid_snap_size_ceil_not_shrunk() {
    init_layered_options();
    let graph = create_graph();
    set_node_property(&graph, LayeredOptions::GRID_SNAP_GRID_SIZE, GRID);
    set_node_property(&graph, CoreOptions::PADDING, ElkPadding::with_any(GRID));

    let n1 = create_grid_node(&graph, 37.0, 53.0);
    set_node_property(&n1, CoreOptions::NODE_SIZE_MINIMUM, KVector::with_values(37.0, 53.0));
    let n2 = create_grid_node(&graph, 41.0, 27.0);
    set_node_property(&n2, CoreOptions::NODE_SIZE_MINIMUM, KVector::with_values(41.0, 27.0));
    create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );

    run_layout(&graph);

    let (_, _, w1, h1) = node_bounds(&n1);
    assert!(w1 >= 37.0, "node width {w1} shrunk below original 37.0");
    assert!(h1 >= 53.0, "node height {h1} shrunk below original 53.0");
    assert!(
        is_grid_multiple(w1, GRID),
        "node width={w1} not grid-aligned"
    );
    assert!(
        is_grid_multiple(h1, GRID),
        "node height={h1} not grid-aligned"
    );

    let (_, _, w2, h2) = node_bounds(&n2);
    assert!(w2 >= 41.0, "node width {w2} shrunk below original 41.0");
    assert!(h2 >= 27.0, "node height {h2} shrunk below original 27.0");
}

#[test]
fn grid_snap_port_boundary_aligned() {
    init_layered_options();
    let graph = create_graph();
    set_node_property(&graph, LayeredOptions::GRID_SNAP_GRID_SIZE, GRID);
    set_node_property(&graph, CoreOptions::PADDING, ElkPadding::with_any(GRID));

    let n1 = create_grid_node(&graph, 37.0, 53.0);
    let n2 = create_grid_node(&graph, 41.0, 27.0);
    create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );

    run_layout(&graph);

    let (x1, y1, w1, h1) = node_bounds(&n1);
    assert!(is_grid_multiple(x1, GRID), "n1.x={x1} not grid-aligned");
    assert!(is_grid_multiple(y1, GRID), "n1.y={y1} not grid-aligned");
    // The right edge of n1 (x1+w1) should also be grid-aligned since both x1 and w1 are
    assert!(
        is_grid_multiple(x1 + w1, GRID),
        "n1 right edge {} not grid-aligned",
        x1 + w1
    );
    assert!(
        is_grid_multiple(y1 + h1, GRID),
        "n1 bottom edge {} not grid-aligned",
        y1 + h1
    );
}

#[test]
fn grid_snap_full_pipeline_up() {
    let graph = layout_grid_graph(Direction::Up);
    assert_all_nodes_grid_aligned(&graph, GRID);
}

#[test]
fn grid_snap_zero_grid_size_no_effect() {
    init_layered_options();
    let graph = create_graph();
    set_node_property(&graph, LayeredOptions::GRID_SNAP_GRID_SIZE, 0.0_f64);

    let n1 = create_node(&graph, 37.0, 53.0);
    let n2 = create_node(&graph, 41.0, 27.0);
    create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );

    run_layout(&graph);

    let (_, _, w1, h1) = node_bounds(&n1);
    assert_eq!(w1, 37.0, "n1 width should be unchanged with grid_size=0");
    assert_eq!(h1, 53.0, "n1 height should be unchanged with grid_size=0");
}

#[test]
fn grid_snap_different_grid_size() {
    init_layered_options();
    let grid = 5.0;
    let graph = create_graph();
    set_node_property(&graph, LayeredOptions::GRID_SNAP_GRID_SIZE, grid);
    set_node_property(&graph, CoreOptions::PADDING, ElkPadding::with_any(grid));

    let n1 = create_grid_node(&graph, 37.0, 53.0);
    let n2 = create_grid_node(&graph, 41.0, 27.0);
    create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );

    run_layout(&graph);

    assert_all_nodes_grid_aligned(&graph, grid);
}

#[test]
fn grid_snap_already_aligned_unchanged() {
    init_layered_options();
    let graph = create_graph();
    set_node_property(&graph, LayeredOptions::GRID_SNAP_GRID_SIZE, GRID);
    set_node_property(&graph, CoreOptions::PADDING, ElkPadding::with_any(GRID));

    // Sizes already grid-aligned
    let n1 = create_grid_node(&graph, 40.0, 60.0);
    set_node_property(&n1, CoreOptions::NODE_SIZE_MINIMUM, KVector::with_values(40.0, 60.0));
    let n2 = create_grid_node(&graph, 50.0, 30.0);
    set_node_property(&n2, CoreOptions::NODE_SIZE_MINIMUM, KVector::with_values(50.0, 30.0));
    create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );

    run_layout(&graph);

    // Sizes should remain exactly as set (already grid multiples)
    let (_, _, w1, h1) = node_bounds(&n1);
    assert_eq!(w1, 40.0, "n1 width should stay 40.0");
    assert_eq!(h1, 60.0, "n1 height should stay 60.0");

    let (_, _, w2, h2) = node_bounds(&n2);
    assert_eq!(w2, 50.0, "n2 width should stay 50.0");
    assert_eq!(h2, 30.0, "n2 height should stay 30.0");

    assert_all_nodes_grid_aligned(&graph, GRID);
}

#[test]
fn grid_snap_disconnected_components() {
    init_layered_options();
    let graph = create_graph();
    set_node_property(&graph, LayeredOptions::GRID_SNAP_GRID_SIZE, GRID);
    set_node_property(&graph, CoreOptions::PADDING, ElkPadding::with_any(GRID));

    // Component 1: n1 → n2
    let n1 = create_grid_node(&graph, 37.0, 53.0);
    let n2 = create_grid_node(&graph, 41.0, 27.0);
    create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );

    // Component 2: n3 → n4 (disconnected from component 1)
    let n3 = create_grid_node(&graph, 23.0, 45.0);
    let n4 = create_grid_node(&graph, 33.0, 19.0);
    create_edge(
        ElkConnectableShapeRef::Node(n3.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );

    run_layout(&graph);

    // All nodes in both components should be grid-aligned
    assert_all_nodes_grid_aligned(&graph, GRID);
}

#[test]
fn grid_snap_fan_out_topology() {
    init_layered_options();
    let graph = create_graph();
    set_node_property(&graph, LayeredOptions::GRID_SNAP_GRID_SIZE, GRID);
    set_node_property(&graph, CoreOptions::PADDING, ElkPadding::with_any(GRID));

    // Fan-out: source → {t1, t2, t3} — puts t1, t2, t3 in the same layer
    let source = create_grid_node(&graph, 37.0, 53.0);
    let t1 = create_grid_node(&graph, 29.0, 41.0);
    let t2 = create_grid_node(&graph, 33.0, 27.0);
    let t3 = create_grid_node(&graph, 23.0, 45.0);

    for t in [&t1, &t2, &t3] {
        create_edge(
            ElkConnectableShapeRef::Node(source.clone()),
            ElkConnectableShapeRef::Node(t.clone()),
        );
    }

    run_layout(&graph);

    assert_all_nodes_grid_aligned(&graph, GRID);
}

#[test]
fn grid_snap_diamond_topology() {
    init_layered_options();
    let graph = create_graph();
    set_node_property(&graph, LayeredOptions::GRID_SNAP_GRID_SIZE, GRID);
    set_node_property(&graph, CoreOptions::PADDING, ElkPadding::with_any(GRID));

    // Diamond: a → {b, c} → d
    let a = create_grid_node(&graph, 37.0, 53.0);
    let b = create_grid_node(&graph, 29.0, 41.0);
    let c = create_grid_node(&graph, 33.0, 27.0);
    let d = create_grid_node(&graph, 23.0, 45.0);

    create_edge(
        ElkConnectableShapeRef::Node(a.clone()),
        ElkConnectableShapeRef::Node(b.clone()),
    );
    create_edge(
        ElkConnectableShapeRef::Node(a.clone()),
        ElkConnectableShapeRef::Node(c.clone()),
    );
    create_edge(
        ElkConnectableShapeRef::Node(b.clone()),
        ElkConnectableShapeRef::Node(d.clone()),
    );
    create_edge(
        ElkConnectableShapeRef::Node(c.clone()),
        ElkConnectableShapeRef::Node(d.clone()),
    );

    run_layout(&graph);

    assert_all_nodes_grid_aligned(&graph, GRID);
}

#[test]
fn grid_snap_larger_graph() {
    init_layered_options();
    let graph = create_graph();
    set_node_property(&graph, LayeredOptions::GRID_SNAP_GRID_SIZE, GRID);
    set_node_property(&graph, CoreOptions::PADDING, ElkPadding::with_any(GRID));

    // 7-node graph spanning multiple layers with varied sizes
    let n1 = create_grid_node(&graph, 37.0, 53.0);
    let n2 = create_grid_node(&graph, 41.0, 27.0);
    let n3 = create_grid_node(&graph, 23.0, 45.0);
    let n4 = create_grid_node(&graph, 51.0, 19.0);
    let n5 = create_grid_node(&graph, 33.0, 67.0);
    let n6 = create_grid_node(&graph, 29.0, 31.0);
    let n7 = create_grid_node(&graph, 47.0, 23.0);

    // n1 → {n2, n3}, n2 → {n4, n5}, n3 → n5, n4 → {n6, n7}, n5 → n7
    for (s, t) in [
        (&n1, &n2), (&n1, &n3),
        (&n2, &n4), (&n2, &n5),
        (&n3, &n5),
        (&n4, &n6), (&n4, &n7),
        (&n5, &n7),
    ] {
        create_edge(
            ElkConnectableShapeRef::Node(s.clone()),
            ElkConnectableShapeRef::Node(t.clone()),
        );
    }

    run_layout(&graph);

    assert_all_nodes_grid_aligned(&graph, GRID);
}

#[test]
fn grid_snap_isolated_node() {
    init_layered_options();
    let graph = create_graph();
    set_node_property(&graph, LayeredOptions::GRID_SNAP_GRID_SIZE, GRID);
    set_node_property(&graph, CoreOptions::PADDING, ElkPadding::with_any(GRID));

    // Single node with no edges
    let _n1 = create_grid_node(&graph, 37.0, 53.0);

    run_layout(&graph);

    assert_all_nodes_grid_aligned(&graph, GRID);
}

#[test]
fn grid_snap_no_effect_without_property() {
    init_layered_options();
    let graph = create_graph();
    // No GRID_SNAP_GRID_SIZE set

    let n1 = create_node(&graph, 37.0, 53.0);
    let n2 = create_node(&graph, 41.0, 27.0);
    create_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );

    run_layout(&graph);

    // Sizes should remain as originally set (no ceil snap)
    let (_, _, w1, h1) = node_bounds(&n1);
    assert_eq!(w1, 37.0, "n1 width should be unchanged");
    assert_eq!(h1, 53.0, "n1 height should be unchanged");
}
