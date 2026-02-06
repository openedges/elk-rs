mod elkt_test_loader;
mod issue_support;

use elkt_test_loader::{find_node_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_layout, set_node_property};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    CrossingMinimizationStrategy, CycleBreakingStrategy, GreedySwitchType, LayeredOptions,
    OrderingStrategy,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::Direction;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const EPSILON: f64 = 0.1;

#[test]
fn issue_871_feedback_edge_below_aligns_nodes() {
    init_layered_options();

    let graph = load_issue_871_graph("issue_871_feedback_below.elkt");
    configure_issue_871_base_options(&graph, true);

    run_layout(&graph);

    let n2 = find_node_by_identifier(&graph, "n2").expect("n2 should exist");
    let n4 = find_node_by_identifier(&graph, "n4").expect("n4 should exist");

    let n2_y = node_y(&n2);
    let n4_y = node_y(&n4);
    assert!(
        (n2_y - n4_y).abs() <= EPSILON,
        "n4 and n2 should align (n2={n2_y}, n4={n4_y})"
    );
}

#[test]
fn issue_871_feedback_edge_basic_aligns_nodes() {
    init_layered_options();

    let graph = load_issue_871_graph("issue_871_feedback_basic.elkt");
    configure_issue_871_base_options(&graph, true);

    run_layout(&graph);

    let n2 = find_node_by_identifier(&graph, "n2").expect("n2 should exist");
    let n3 = find_node_by_identifier(&graph, "n3").expect("n3 should exist");

    let n2_y = node_y(&n2);
    let n3_y = node_y(&n3);
    assert!(
        (n2_y - n3_y).abs() <= EPSILON,
        "n3 and n2 should align (n2={n2_y}, n3={n3_y})"
    );
}

#[test]
fn issue_871_model_order_without_feedback_edges_keeps_expected_node_positions() {
    init_layered_options();

    let graph = load_issue_871_graph("issue_871_model_order.elkt");
    configure_issue_871_base_options(&graph, false);

    run_layout(&graph);

    let n1 = find_node_by_identifier(&graph, "n1").expect("n1 should exist");
    let n2 = find_node_by_identifier(&graph, "n2").expect("n2 should exist");
    let n3 = find_node_by_identifier(&graph, "n3").expect("n3 should exist");
    let n4 = find_node_by_identifier(&graph, "n4").expect("n4 should exist");

    let (n1_x, n1_y) = node_xy(&n1);
    let (n2_x, n2_y) = node_xy(&n2);
    let (n3_x, n3_y) = node_xy(&n3);
    let (n4_x, n4_y) = node_xy(&n4);

    assert!(
        n1_x < n2_x
            && n2_x < n4_x
            && n3_x <= n2_x
            && (n2_y - n4_y).abs() <= EPSILON
            && n3_y > n1_y,
        "unexpected coordinates n1=({n1_x},{n1_y}) n2=({n2_x},{n2_y}) n3=({n3_x},{n3_y}) n4=({n4_x},{n4_y})"
    );
}

fn load_issue_871_graph(file_name: &str) -> ElkNodeRef {
    let path = format!(
        "{}/tests/resources/issues/{file_name}",
        env!("CARGO_MANIFEST_DIR")
    );
    load_layered_graph_from_elkt(&path).expect("issue_871 resource should load")
}

fn configure_issue_871_base_options(graph: &ElkNodeRef, enable_feedback_edges: bool) {
    set_node_property(graph, CoreOptions::DIRECTION, Direction::Right);
    set_node_property(
        graph,
        LayeredOptions::CYCLE_BREAKING_STRATEGY,
        CycleBreakingStrategy::ModelOrder,
    );
    set_node_property(
        graph,
        LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY,
        OrderingStrategy::PreferEdges,
    );
    set_node_property(
        graph,
        LayeredOptions::CROSSING_MINIMIZATION_STRATEGY,
        CrossingMinimizationStrategy::None,
    );
    set_node_property(
        graph,
        LayeredOptions::CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE,
        GreedySwitchType::Off,
    );
    set_node_property(graph, CoreOptions::PADDING, ElkPadding::with_any(0.0));
    set_node_property(graph, CoreOptions::SPACING_NODE_NODE, 10.0);
    set_node_property(graph, LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS, 20.0);

    if enable_feedback_edges {
        set_node_property(graph, LayeredOptions::FEEDBACK_EDGES, true);
    }
}

fn node_xy(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y())
}

fn node_y(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().y()
}
