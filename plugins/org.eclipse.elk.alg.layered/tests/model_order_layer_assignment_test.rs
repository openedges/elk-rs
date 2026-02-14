use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    CycleBreakingStrategy, LayeredOptions, LayeringStrategy,
    NodePromotionStrategy, OrderingStrategy,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{core_options::CoreOptions, Direction};
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkGraphElementRef, ElkNodeRef,
};

const POSITION_EPSILON: f64 = 0.1;
const ABSOLUTE_PARITY_EPSILON: f64 = 5.0;

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .set_dimensions(width, height);
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: T,
) {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn node_position(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y())
}

fn add_node_label(node: &ElkNodeRef, text: &str) {
    let _ = ElkGraphUtil::create_label_with_text(
        text,
        Some(ElkGraphElementRef::Node(node.clone())),
    );
}

fn assert_less(actual: f64, reference: f64, message: &str) {
    assert!(actual < reference, "{message}: expected {actual} < {reference}");
}

fn assert_approx_eq(actual: f64, expected: f64, message: &str) {
    assert!(
        (actual - expected).abs() <= POSITION_EPSILON,
        "{message}: expected {expected}, got {actual}"
    );
}

fn assert_approx_eq_within(actual: f64, expected: f64, epsilon: f64, message: &str) {
    assert!(
        (actual - expected).abs() <= epsilon,
        "{message}: expected {expected}, got {actual} (eps={epsilon})"
    );
}

fn normalized_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let min_x = points
        .iter()
        .map(|(x, _)| *x)
        .fold(f64::INFINITY, f64::min);
    let min_y = points
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::INFINITY, f64::min);
    points
        .iter()
        .map(|(x, y)| (x - min_x, y - min_y))
        .collect()
}

fn run_layout_for_graph(
    root: &ElkNodeRef,
    promotion: NodePromotionStrategy,
    layering: LayeringStrategy,
) {
    set_node_property(root, CoreOptions::ALGORITHM, LayeredOptions::ALGORITHM_ID.to_string());
    set_node_property(root, LayeredOptions::LAYERING_NODE_PROMOTION_STRATEGY, promotion);
    set_node_property(root, LayeredOptions::LAYERING_STRATEGY, layering);
    set_node_property(
        root,
        LayeredOptions::CYCLE_BREAKING_STRATEGY,
        CycleBreakingStrategy::ModelOrder,
    );
    set_node_property(
        root,
        LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY,
        OrderingStrategy::PreferEdges,
    );
    set_node_property(root, CoreOptions::DIRECTION, Direction::Right);
    set_node_property(root, CoreOptions::PADDING, ElkPadding::new());
    set_node_property(root, CoreOptions::SPACING_NODE_NODE, 10.0_f64);
    set_node_property(
        root,
        LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS,
        20.0_f64,
    );

    let mut provider = LayeredLayoutProvider::new();
    provider.layout(root, &mut BasicProgressMonitor::new());
}

fn build_forward_promotion_graph() -> (ElkNodeRef, Vec<ElkNodeRef>) {
    let parent = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n2 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n3 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n4 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n5 = ElkGraphUtil::create_node(Some(parent.clone()));

    for node in [&n1, &n2, &n3, &n4, &n5] {
        set_dimensions(node, 30.0, 30.0);
    }
    add_node_label(&n1, "n1");
    add_node_label(&n2, "n2");
    add_node_label(&n3, "n3");
    add_node_label(&n4, "n4");
    add_node_label(&n5, "n5");

    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n2.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n3.clone()),
        ElkConnectableShapeRef::Node(n5.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n4.clone()),
        ElkConnectableShapeRef::Node(n5.clone()),
    );
    (parent, vec![n1, n2, n3, n4, n5])
}

fn build_backward_promotion_graph() -> (ElkNodeRef, Vec<ElkNodeRef>) {
    let parent = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n2 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n3 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n4 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n5 = ElkGraphUtil::create_node(Some(parent.clone()));

    for node in [&n1, &n2, &n3, &n4, &n5] {
        set_dimensions(node, 30.0, 30.0);
    }
    add_node_label(&n1, "n1");
    add_node_label(&n2, "n2");
    add_node_label(&n3, "n3");
    add_node_label(&n4, "n4");
    add_node_label(&n5, "n5");

    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n2.clone()),
        ElkConnectableShapeRef::Node(n5.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n2.clone()),
        ElkConnectableShapeRef::Node(n5.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n3.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n4.clone()),
        ElkConnectableShapeRef::Node(n5.clone()),
    );
    (parent, vec![n1, n2, n3, n4, n5])
}

fn build_forward_more_nodes_graph() -> (ElkNodeRef, Vec<ElkNodeRef>) {
    let parent = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n2 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n3 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n4 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n5 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n6 = ElkGraphUtil::create_node(Some(parent.clone()));

    for node in [&n1, &n2, &n3, &n4, &n5, &n6] {
        set_dimensions(node, 30.0, 30.0);
    }
    add_node_label(&n1, "n1");
    add_node_label(&n2, "n2");
    add_node_label(&n3, "n3");
    add_node_label(&n4, "n4");
    add_node_label(&n5, "n5");
    add_node_label(&n6, "n6");

    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n5.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n2.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n3.clone()),
        ElkConnectableShapeRef::Node(n6.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n4.clone()),
        ElkConnectableShapeRef::Node(n6.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n5.clone()),
        ElkConnectableShapeRef::Node(n6.clone()),
    );

    (parent, vec![n1, n2, n3, n4, n5, n6])
}

fn build_backward_more_nodes_graph() -> (ElkNodeRef, Vec<ElkNodeRef>) {
    let parent = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n2 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n3 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n4 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n5 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n6 = ElkGraphUtil::create_node(Some(parent.clone()));

    for node in [&n1, &n2, &n3, &n4, &n5, &n6] {
        set_dimensions(node, 30.0, 30.0);
    }
    add_node_label(&n1, "n1");
    add_node_label(&n2, "n2");
    add_node_label(&n3, "n3");
    add_node_label(&n4, "n4");
    add_node_label(&n5, "n5");
    add_node_label(&n6, "n6");

    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n2.clone()),
        ElkConnectableShapeRef::Node(n6.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n2.clone()),
        ElkConnectableShapeRef::Node(n6.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n3.clone()),
        ElkConnectableShapeRef::Node(n6.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n4.clone()),
        ElkConnectableShapeRef::Node(n5.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n5.clone()),
        ElkConnectableShapeRef::Node(n6.clone()),
    );

    (parent, vec![n1, n2, n3, n4, n5, n6])
}

fn build_forward_no_end_node_graph() -> (ElkNodeRef, Vec<ElkNodeRef>) {
    let parent = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n2 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n3 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n4 = ElkGraphUtil::create_node(Some(parent.clone()));

    for node in [&n1, &n2, &n3, &n4] {
        set_dimensions(node, 30.0, 30.0);
    }
    add_node_label(&n1, "n1");
    add_node_label(&n2, "n2");
    add_node_label(&n3, "n3");
    add_node_label(&n4, "n4");

    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n2.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );

    (parent, vec![n1, n2, n3, n4])
}

fn build_backward_no_end_node2_graph() -> (ElkNodeRef, Vec<ElkNodeRef>) {
    let parent = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n2 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n3 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n4 = ElkGraphUtil::create_node(Some(parent.clone()));

    for node in [&n1, &n2, &n3, &n4] {
        set_dimensions(node, 30.0, 30.0);
    }
    add_node_label(&n1, "n1");
    add_node_label(&n2, "n2");
    add_node_label(&n3, "n3");
    add_node_label(&n4, "n4");

    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n3.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );

    (parent, vec![n1, n2, n3, n4])
}

fn build_backward_no_end_node_graph() -> (ElkNodeRef, Vec<ElkNodeRef>) {
    let parent = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n2 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n3 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n4 = ElkGraphUtil::create_node(Some(parent.clone()));

    for node in [&n1, &n2, &n3, &n4] {
        set_dimensions(node, 30.0, 30.0);
    }
    add_node_label(&n1, "n1");
    add_node_label(&n2, "n2");
    add_node_label(&n3, "n3");
    add_node_label(&n4, "n4");

    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n2.clone()),
        ElkConnectableShapeRef::Node(n3.clone()),
    );
    let _ = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n3.clone()),
        ElkConnectableShapeRef::Node(n4.clone()),
    );

    (parent, vec![n1, n2, n3, n4])
}

#[test]
fn model_order_layer_assignment_forward_promotion_runs() {
    initialize_plain_java_layout();
    let (root, nodes) = build_forward_promotion_graph();
    run_layout_for_graph(
        &root,
        NodePromotionStrategy::ModelOrderLeftToRight,
        LayeringStrategy::LongestPathSource,
    );
    let (n1x, n1y) = node_position(&nodes[0]);
    let (n2x, n2y) = node_position(&nodes[1]);
    let (n3x, _) = node_position(&nodes[2]);
    let (n4x, n4y) = node_position(&nodes[3]);
    let (n5x, _) = node_position(&nodes[4]);

    assert_approx_eq(n1x, 0.0, "n1 should stay at left boundary");
    assert_less(n1x, n2x, "n1 should be left of n2");
    assert_less(n2x, n3x, "n2 should be left of n3");
    assert_less(n3x, n5x, "n3 should be left of n5");
    assert_approx_eq(n3x, n4x, "forward promotion should align n3 and n4");
    assert_less(n2y, n4y, "n4 should be below n2");
    assert!(n1y >= 0.0, "n1 y should stay non-negative");
}

#[test]
fn model_order_layer_assignment_backward_promotion_runs() {
    initialize_plain_java_layout();
    let (root, nodes) = build_backward_promotion_graph();
    run_layout_for_graph(
        &root,
        NodePromotionStrategy::ModelOrderRightToLeft,
        LayeringStrategy::LongestPath,
    );
    let (n1x, n1y) = node_position(&nodes[0]);
    let (n2x, n2y) = node_position(&nodes[1]);
    let (n3x, n3y) = node_position(&nodes[2]);
    let (n4x, n4y) = node_position(&nodes[3]);
    let (n5x, _) = node_position(&nodes[4]);

    assert_approx_eq(n1x, 0.0, "n1 should stay at left boundary");
    assert_less(n1x, n3x, "n1 should be left of n3");
    assert_approx_eq(n2x, n3x, "backward promotion should align n2 and n3");
    assert_less(n2x, n5x, "n2 should be left of n5");
    assert_less(n2x, n4x, "n4 should be right of n2");
    assert_less(n2y, n4y, "n4 should be below n2");
    assert_less(n1y, n3y, "n1 should be above n3");
}

#[test]
fn model_order_layer_assignment_forward_more_nodes_preserves_relative_order() {
    initialize_plain_java_layout();
    let (root, nodes) = build_forward_more_nodes_graph();
    run_layout_for_graph(
        &root,
        NodePromotionStrategy::ModelOrderLeftToRight,
        LayeringStrategy::LongestPathSource,
    );

    let (n1x, _) = node_position(&nodes[0]);
    let (n2x, n2y) = node_position(&nodes[1]);
    let (n3x, _) = node_position(&nodes[2]);
    let (n4x, n4y) = node_position(&nodes[3]);
    let (n5x, n5y) = node_position(&nodes[4]);
    let (n6x, _) = node_position(&nodes[5]);

    assert_approx_eq(n1x, 0.0, "n1 should stay at left boundary");
    assert_less(n1x, n2x, "n1 should be left of n2");
    assert_less(n2x, n3x, "n2 should be left of n3");
    assert_less(n3x, n6x, "n3 should be left of n6");
    assert_less(n4x, n6x, "n4 should be left of n6");
    assert_less(n5x, n6x, "n5 should be left of n6");
    assert_less(n2y, n4y, "n4 should be below n2");
    assert_less(n4y, n5y, "n5 should be below n4");
}

#[test]
fn model_order_layer_assignment_backward_more_nodes_preserves_relative_order() {
    initialize_plain_java_layout();
    let (root, nodes) = build_backward_more_nodes_graph();
    run_layout_for_graph(
        &root,
        NodePromotionStrategy::ModelOrderRightToLeft,
        LayeringStrategy::LongestPath,
    );

    let (n1x, _) = node_position(&nodes[0]);
    let (n2x, n2y) = node_position(&nodes[1]);
    let (n3x, n3y) = node_position(&nodes[2]);
    let (n4x, n4y) = node_position(&nodes[3]);
    let (n5x, _) = node_position(&nodes[4]);
    let (n6x, _) = node_position(&nodes[5]);

    assert_approx_eq(n1x, 0.0, "n1 should stay at left boundary");
    assert_less(n1x, n2x, "n1 should be left of n2");
    assert_less(n1x, n3x, "n1 should be left of n3");
    assert_less(n1x, n4x, "n1 should be left of n4");
    assert_less(n4x, n5x, "n4 should be left of n5");
    assert_less(n5x, n6x, "n5 should be left of n6");
    assert_less(n2y, n3y, "n3 should be below n2");
    assert_less(n3y, n4y, "n4 should be below n3");
}

#[test]
fn model_order_layer_assignment_forward_no_end_node_preserves_relative_order() {
    initialize_plain_java_layout();
    let (root, nodes) = build_forward_no_end_node_graph();
    run_layout_for_graph(
        &root,
        NodePromotionStrategy::ModelOrderLeftToRight,
        LayeringStrategy::LongestPathSource,
    );

    let (n1x, _) = node_position(&nodes[0]);
    let (n2x, n2y) = node_position(&nodes[1]);
    let (n3x, n3y) = node_position(&nodes[2]);
    let (n4x, n4y) = node_position(&nodes[3]);

    assert_approx_eq(n1x, 0.0, "n1 should stay at left boundary");
    assert_less(n1x, n2x, "n1 should be left of n2");
    assert_less(n2x, n3x, "n2 should be left of n3");
    assert_approx_eq(n3x, n4x, "n3 and n4 should align in x");
    assert_approx_eq(n2y, n3y, "n2 and n3 should stay on same row");
    assert_less(n3y, n4y, "n4 should be below n3");
}

#[test]
fn model_order_layer_assignment_backward_no_end_node_preserves_relative_order() {
    initialize_plain_java_layout();
    let (root, nodes) = build_backward_no_end_node_graph();
    run_layout_for_graph(
        &root,
        NodePromotionStrategy::ModelOrderRightToLeft,
        LayeringStrategy::LongestPath,
    );

    let (n1x, n1y) = node_position(&nodes[0]);
    let (n2x, n2y) = node_position(&nodes[1]);
    let (n3x, n3y) = node_position(&nodes[2]);
    let (n4x, n4y) = node_position(&nodes[3]);
    let normalized = normalized_points(&[(n1x, n1y), (n2x, n2y), (n3x, n3y), (n4x, n4y)]);

    assert_less(n1x, n4x, "n1 should stay left of n4");
    assert_less(n2x, n3x, "n3 should be right of n2");
    assert_less(n1y, n2y, "n2 should be below n1");
    assert_less(n3x, n4x, "n4 should be right of n3");
    assert_approx_eq(n2y, n3y, "n2 and n3 should stay on same row");
    assert_less(n1y, n4y, "n4 should be below n1");
    assert_less(n4y, n2y, "n4 should be above n2");

    // Java baseline normalized grid for this shape is x=[0,0,50,100], y=[0,6,40,40].
    let mut normalized_x: Vec<f64> = normalized.iter().map(|(x, _)| *x).collect();
    normalized_x.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pattern_java_like = (normalized_x[0] - 0.0).abs() <= ABSOLUTE_PARITY_EPSILON
        && (normalized_x[1] - 0.0).abs() <= ABSOLUTE_PARITY_EPSILON
        && (normalized_x[2] - 50.0).abs() <= ABSOLUTE_PARITY_EPSILON
        && (normalized_x[3] - 100.0).abs() <= ABSOLUTE_PARITY_EPSILON;
    assert!(
        pattern_java_like,
        "unexpected normalized x columns: {:?}",
        normalized_x
    );

    let mut normalized_y: Vec<f64> = normalized.iter().map(|(_, y)| *y).collect();
    normalized_y.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    assert_approx_eq_within(normalized_y[0], 0.0, ABSOLUTE_PARITY_EPSILON, "grid y0");
    assert_approx_eq_within(normalized_y[1], 6.0, ABSOLUTE_PARITY_EPSILON, "grid y1");
    assert_approx_eq_within(normalized_y[2], 40.0, ABSOLUTE_PARITY_EPSILON, "grid y2");
    assert_approx_eq_within(normalized_y[3], 40.0, ABSOLUTE_PARITY_EPSILON, "grid y3");
}

#[test]
fn model_order_layer_assignment_backward_no_end_node2_preserves_relative_order() {
    initialize_plain_java_layout();
    let (root, nodes) = build_backward_no_end_node2_graph();
    run_layout_for_graph(
        &root,
        NodePromotionStrategy::ModelOrderRightToLeft,
        LayeringStrategy::LongestPath,
    );

    let (n1x, n1y) = node_position(&nodes[0]);
    let (n2x, n2y) = node_position(&nodes[1]);
    let (n3x, n3y) = node_position(&nodes[2]);
    let (n4x, n4y) = node_position(&nodes[3]);
    let normalized = normalized_points(&[(n1x, n1y), (n2x, n2y), (n3x, n3y), (n4x, n4y)]);

    assert_approx_eq(n1x, 0.0, "n1 should stay at left boundary");
    assert_less(n1x, n2x, "n1 should be left of n2");
    assert_approx_eq(n2x, n3x, "n2 and n3 should align in x");
    assert_less(n3x, n4x, "n4 should be right of n3");
    assert_less(n1y, n3y, "n1 should be above n3");
    assert_less(n2y, n3y, "n2 should be above n3");
    assert_approx_eq(n3y, n4y, "n3 and n4 should stay on same row");

    // Java baseline normalized grid for this shape is x=[0,50,50,100].
    let mut normalized_x: Vec<f64> = normalized.iter().map(|(x, _)| *x).collect();
    normalized_x.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pattern_java_like = (normalized_x[0] - 0.0).abs() <= ABSOLUTE_PARITY_EPSILON
        && (normalized_x[1] - 50.0).abs() <= ABSOLUTE_PARITY_EPSILON
        && (normalized_x[2] - 50.0).abs() <= ABSOLUTE_PARITY_EPSILON
        && (normalized_x[3] - 100.0).abs() <= ABSOLUTE_PARITY_EPSILON;
    assert!(
        pattern_java_like,
        "unexpected normalized x columns: {:?}",
        normalized_x
    );

    let mut normalized_y: Vec<f64> = normalized.iter().map(|(_, y)| *y).collect();
    normalized_y.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    assert_approx_eq_within(normalized_y[0], 0.0, ABSOLUTE_PARITY_EPSILON, "grid y0");
    assert_approx_eq_within(normalized_y[1], 5.0, 15.0, "grid y1");
    assert_approx_eq_within(normalized_y[2], 40.0, ABSOLUTE_PARITY_EPSILON, "grid y2");
    assert_approx_eq_within(normalized_y[3], 40.0, ABSOLUTE_PARITY_EPSILON, "grid y3");
}
