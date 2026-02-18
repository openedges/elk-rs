use std::sync::LazyLock;

use org_eclipse_elk_alg_topdownpacking::org::eclipse::elk::alg::topdownpacking::options::{
    TopdownpackingMetaDataProvider, TopdownpackingOptions,
};
use org_eclipse_elk_alg_topdownpacking::org::eclipse::elk::alg::topdownpacking::topdownpacking_layout_provider::TopdownpackingLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::topdown_layout_provider::ITopdownLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const EPS: f64 = 1.0e-5;

fn init_topdownpacking_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&TopdownpackingMetaDataProvider);
}

#[test]
fn test_empty_graph() {
    init_topdownpacking_options();
    let graph = create_graph(0);
    let mut provider = TopdownpackingLayoutProvider::new();
    let mut monitor = BasicProgressMonitor::new();
    provider.layout(&graph, &mut monitor);
}

#[test]
fn test_two_nodes() {
    init_topdownpacking_options();
    let graph = create_graph(2);
    let hierarchical_width = get_graph_property(
        &graph,
        TopdownpackingOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH,
    );
    let hierarchical_aspect_ratio = get_graph_property(
        &graph,
        TopdownpackingOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO,
    );
    let padding: ElkPadding = get_graph_property(&graph, TopdownpackingOptions::PADDING);
    let node_node_spacing = get_graph_property(&graph, TopdownpackingOptions::SPACING_NODE_NODE);

    let mut provider = TopdownpackingLayoutProvider::new();
    let predicted_size = provider.get_predicted_graph_size(&graph);
    let expected_width =
        padding.left + 2.0 * hierarchical_width + node_node_spacing + padding.right;
    let expected_height =
        padding.top + hierarchical_width / hierarchical_aspect_ratio + padding.bottom;

    assert_close(predicted_size.x, expected_width, "predicted width");
    assert_close(predicted_size.y, expected_height, "predicted height");

    let mut monitor = BasicProgressMonitor::new();
    provider.layout(&graph, &mut monitor);

    let child_one = child_at(&graph, 0);
    assert_shape(
        &child_one,
        padding.left,
        padding.top,
        hierarchical_width,
        hierarchical_width / hierarchical_aspect_ratio,
        "child one",
    );

    let child_two = child_at(&graph, 1);
    assert_shape(
        &child_two,
        padding.left + hierarchical_width + node_node_spacing,
        padding.top,
        hierarchical_width,
        hierarchical_width / hierarchical_aspect_ratio,
        "child two",
    );
}

#[test]
fn test_three_nodes() {
    init_topdownpacking_options();
    let graph = create_graph(3);
    let hierarchical_width = get_graph_property(
        &graph,
        TopdownpackingOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH,
    );
    let hierarchical_aspect_ratio = get_graph_property(
        &graph,
        TopdownpackingOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO,
    );
    let padding: ElkPadding = get_graph_property(&graph, TopdownpackingOptions::PADDING);
    let node_node_spacing = get_graph_property(&graph, TopdownpackingOptions::SPACING_NODE_NODE);

    let mut provider = TopdownpackingLayoutProvider::new();
    let predicted_size = provider.get_predicted_graph_size(&graph);
    let expected_width =
        padding.left + 2.0 * hierarchical_width + node_node_spacing + padding.right;
    let expected_height = padding.top
        + 2.0 * (hierarchical_width / hierarchical_aspect_ratio)
        + node_node_spacing
        + padding.bottom;

    assert_close(predicted_size.x, expected_width, "predicted width");
    assert_close(predicted_size.y, expected_height, "predicted height");

    let mut monitor = BasicProgressMonitor::new();
    provider.layout(&graph, &mut monitor);

    let child_one = child_at(&graph, 0);
    assert_shape(
        &child_one,
        padding.left,
        padding.top,
        hierarchical_width,
        hierarchical_width / hierarchical_aspect_ratio,
        "child one",
    );

    let child_two = child_at(&graph, 1);
    assert_shape(
        &child_two,
        padding.left + hierarchical_width + node_node_spacing,
        padding.top,
        hierarchical_width,
        hierarchical_width / hierarchical_aspect_ratio,
        "child two",
    );

    let child_three = child_at(&graph, 2);
    assert_shape(
        &child_three,
        padding.left,
        padding.top + (hierarchical_width / hierarchical_aspect_ratio) + node_node_spacing,
        2.0 * hierarchical_width + node_node_spacing,
        hierarchical_width / hierarchical_aspect_ratio,
        "child three",
    );
}

#[test]
fn test_five_nodes() {
    init_topdownpacking_options();
    let graph = create_graph(5);
    let hierarchical_width = get_graph_property(
        &graph,
        TopdownpackingOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH,
    );
    let hierarchical_aspect_ratio = get_graph_property(
        &graph,
        TopdownpackingOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO,
    );
    let padding: ElkPadding = get_graph_property(&graph, TopdownpackingOptions::PADDING);
    let node_node_spacing = get_graph_property(&graph, TopdownpackingOptions::SPACING_NODE_NODE);

    let mut provider = TopdownpackingLayoutProvider::new();
    let predicted_size = provider.get_predicted_graph_size(&graph);
    let expected_width =
        padding.left + 3.0 * hierarchical_width + 2.0 * node_node_spacing + padding.right;
    let expected_height = padding.top
        + 2.0 * (hierarchical_width / hierarchical_aspect_ratio)
        + node_node_spacing
        + padding.bottom;

    assert_close(predicted_size.x, expected_width, "predicted width");
    assert_close(predicted_size.y, expected_height, "predicted height");

    let mut monitor = BasicProgressMonitor::new();
    provider.layout(&graph, &mut monitor);

    let expanded_width = hierarchical_width + 0.5 * (hierarchical_width + node_node_spacing);
    let child_four = child_at(&graph, 3);
    assert_shape(
        &child_four,
        padding.left,
        padding.top + (hierarchical_width / hierarchical_aspect_ratio) + node_node_spacing,
        expanded_width,
        hierarchical_width / hierarchical_aspect_ratio,
        "child four",
    );

    let mut monitor = BasicProgressMonitor::new();
    provider.layout(&graph, &mut monitor);
    let child_five = child_at(&graph, 4);
    assert_shape(
        &child_five,
        padding.left + expanded_width + node_node_spacing,
        padding.top + (hierarchical_width / hierarchical_aspect_ratio) + node_node_spacing,
        expanded_width,
        hierarchical_width / hierarchical_aspect_ratio,
        "child five",
    );
}

fn create_graph(number_of_nodes: usize) -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    for _ in 0..number_of_nodes {
        ElkGraphUtil::create_node(Some(graph.clone()));
    }
    graph
}

fn child_at(graph: &ElkNodeRef, index: usize) -> ElkNodeRef {
    let mut graph_mut = graph.borrow_mut();
    graph_mut.children().get(index).expect("child must exist")
}

fn assert_shape(
    node: &ElkNodeRef,
    expected_x: f64,
    expected_y: f64,
    expected_width: f64,
    expected_height: f64,
    context: &str,
) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    assert_close(shape.x(), expected_x, &format!("{context} x"));
    assert_close(shape.y(), expected_y, &format!("{context} y"));
    assert_close(shape.width(), expected_width, &format!("{context} width"));
    assert_close(
        shape.height(),
        expected_height,
        &format!("{context} height"),
    );
}

fn assert_close(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= EPS,
        "{context} mismatch: actual={actual}, expected={expected}, eps={EPS}"
    );
}

fn get_graph_property<T: Clone + Send + Sync + 'static>(
    graph: &ElkNodeRef,
    property: &'static LazyLock<Property<T>>,
) -> T {
    let mut graph_mut = graph.borrow_mut();
    graph_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
        .expect("graph property default should be available")
}
