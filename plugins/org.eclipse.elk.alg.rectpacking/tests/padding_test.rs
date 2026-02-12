use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::options::{
    RectPackingMetaDataProvider, RectPackingOptions,
};
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::rect_packing_layout_provider::RectPackingLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::BasicProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const EPS: f64 = 1.0;

fn init_rectpacking_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&RectPackingMetaDataProvider);
}

#[test]
fn test_top_padding() {
    init_rectpacking_options();
    let (parent, nodes) = create_base_padding_graph(ElkPadding::with_values(1000.0, 0.0, 0.0, 0.0));
    run_layout(&parent);

    assert_graph_size(&parent, 150.0, 1030.0);
    assert_node_position(&nodes[0], 0.0, 1000.0);
    assert_node_position(&nodes[1], 40.0, 1000.0);
    assert_node_position(&nodes[2], 80.0, 1000.0);
    assert_node_position(&nodes[3], 120.0, 1000.0);
}

#[test]
fn test_left_padding() {
    init_rectpacking_options();
    let (parent, nodes) = create_base_padding_graph(ElkPadding::with_values(0.0, 0.0, 0.0, 1000.0));
    run_layout(&parent);

    assert_graph_size(&parent, 1030.0, 150.0);
    assert_node_position(&nodes[0], 1000.0, 0.0);
    assert_node_position(&nodes[1], 1000.0, 40.0);
    assert_node_position(&nodes[2], 1000.0, 80.0);
    assert_node_position(&nodes[3], 1000.0, 120.0);
}

#[test]
fn test_bottom_padding() {
    init_rectpacking_options();
    let (parent, nodes) = create_base_padding_graph(ElkPadding::with_values(0.0, 0.0, 1000.0, 0.0));
    run_layout(&parent);

    assert_graph_size(&parent, 150.0, 1030.0);
    assert_node_position(&nodes[0], 0.0, 0.0);
    assert_node_position(&nodes[1], 40.0, 0.0);
    assert_node_position(&nodes[2], 80.0, 0.0);
    assert_node_position(&nodes[3], 120.0, 0.0);
}

#[test]
fn test_right_padding() {
    init_rectpacking_options();
    let (parent, nodes) = create_base_padding_graph(ElkPadding::with_values(0.0, 1000.0, 0.0, 0.0));
    run_layout(&parent);

    assert_graph_size(&parent, 1030.0, 150.0);
    assert_node_position(&nodes[0], 0.0, 0.0);
    assert_node_position(&nodes[1], 0.0, 40.0);
    assert_node_position(&nodes[2], 0.0, 80.0);
    assert_node_position(&nodes[3], 0.0, 120.0);
}

fn create_base_padding_graph(padding: ElkPadding) -> (ElkNodeRef, Vec<ElkNodeRef>) {
    let parent = ElkGraphUtil::create_graph();
    set_node_property(
        &parent,
        CoreOptions::ALGORITHM,
        RectPackingOptions::ALGORITHM_ID.to_string(),
    );
    set_node_property(&parent, RectPackingOptions::ASPECT_RATIO, 1.3_f64);
    set_node_property(&parent, CoreOptions::SPACING_NODE_NODE, 10.0_f64);
    set_node_property(&parent, CoreOptions::PADDING, padding);

    let mut nodes = Vec::new();
    for _ in 0..4 {
        let node = ElkGraphUtil::create_node(Some(parent.clone()));
        set_dimensions(&node, 30.0, 30.0);
        nodes.push(node);
    }

    (parent, nodes)
}

fn run_layout(parent: &ElkNodeRef) {
    let mut provider = RectPackingLayoutProvider::new();
    let mut monitor = BasicProgressMonitor::new();
    provider.layout(parent, &mut monitor);
}

fn assert_graph_size(node: &ElkNodeRef, expected_width: f64, expected_height: f64) {
    let (actual_width, actual_height) = node_dimensions(node);
    assert_close(actual_width, expected_width, "graph width");
    assert_close(actual_height, expected_height, "graph height");
}

fn assert_node_position(node: &ElkNodeRef, expected_x: f64, expected_y: f64) {
    let (actual_x, actual_y) = node_location(node);
    assert_close(actual_x, expected_x, "node x");
    assert_close(actual_y, expected_y, "node y");
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn node_dimensions(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
}

fn node_location(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y())
}

fn assert_close(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= EPS,
        "{context} mismatch: actual={actual}, expected={expected}, eps={EPS}"
    );
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
