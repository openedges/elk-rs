use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::options::{
    RectPackingMetaDataProvider, RectPackingOptions,
};
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::p1widthapproximation::WidthApproximationStrategy;
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
fn test_place_block_from_next_row_on_top() {
    init_rectpacking_options();

    let parent = create_compaction_graph(110.0, 10.0);
    let n1 = create_node(&parent, 30.0, 70.0);
    let n2 = create_node(&parent, 30.0, 10.0);
    let n3 = create_node(&parent, 30.0, 10.0);
    let n4 = create_node(&parent, 30.0, 50.0);
    let n5 = create_node(&parent, 30.0, 50.0);
    let n6 = create_node(&parent, 100.0, 10.0);
    create_node(&parent, 100.0, 10.0);

    run_layout(&parent);

    assert_graph_size(&parent, 110.0, 110.0);
    assert_node_position(&n1, 0.0, 0.0);
    assert_node_position(&n2, 40.0, 0.0);
    assert_node_position(&n3, 80.0, 0.0);
    assert_node_position(&n4, 40.0, 20.0);
    assert_node_position(&n5, 80.0, 20.0);
    assert_node_position(&n6, 0.0, 80.0);
}

#[test]
fn test_place_block_from_next_row_on_top_does_not_work() {
    init_rectpacking_options();

    let parent = create_compaction_graph(110.0, 10.0);
    let n1 = create_node(&parent, 30.0, 70.0);
    let n2 = create_node(&parent, 40.0, 10.0);
    let n3 = create_node(&parent, 30.0, 10.0);
    let n4 = create_node(&parent, 30.0, 50.0);
    let n5 = create_node(&parent, 30.0, 50.0);
    create_node(&parent, 100.0, 10.0);
    create_node(&parent, 100.0, 10.0);

    run_layout(&parent);

    assert_graph_size(&parent, 100.0, 170.0);
    assert_node_position(&n1, 0.0, 0.0);
    assert_node_position(&n2, 40.0, 0.0);
    assert_node_position(&n3, 40.0, 20.0);
    assert_node_position(&n4, 0.0, 80.0);
    assert_node_position(&n5, 40.0, 80.0);
}

#[test]
fn test_absorb_block() {
    init_rectpacking_options();

    let parent = create_compaction_graph(110.0, 10.0);
    let n1 = create_node(&parent, 30.0, 70.0);
    let n2 = create_node(&parent, 30.0, 30.0);
    let n3 = create_node(&parent, 30.0, 30.0);
    let n4 = create_node(&parent, 30.0, 30.0);
    let n5 = create_node(&parent, 30.0, 30.0);
    let n6 = create_node(&parent, 100.0, 10.0);
    create_node(&parent, 100.0, 10.0);

    run_layout(&parent);

    assert_graph_size(&parent, 110.0, 110.0);
    assert_node_position(&n1, 0.0, 0.0);
    assert_node_position(&n2, 40.0, 0.0);
    assert_node_position(&n3, 80.0, 0.0);
    assert_node_position(&n4, 40.0, 40.0);
    assert_node_position(&n5, 80.0, 40.0);
    assert_node_position(&n6, 0.0, 80.0);
}

#[test]
fn test_compact_block() {
    init_rectpacking_options();

    let parent = create_compaction_graph(190.0, 10.0);
    let n1 = create_node(&parent, 30.0, 70.0);
    let n2 = create_node(&parent, 30.0, 30.0);
    let n3 = create_node(&parent, 30.0, 30.0);
    let n4 = create_node(&parent, 30.0, 30.0);
    let n5 = create_node(&parent, 30.0, 30.0);

    run_layout(&parent);

    assert_graph_size(&parent, 110.0, 70.0);
    assert_node_position(&n1, 0.0, 0.0);
    assert_node_position(&n2, 40.0, 0.0);
    assert_node_position(&n3, 80.0, 0.0);
    assert_node_position(&n4, 40.0, 40.0);
    assert_node_position(&n5, 80.0, 40.0);
}

#[test]
fn test_split_block() {
    init_rectpacking_options();

    let parent = create_compaction_graph(110.0, 10.0);
    let n1 = create_node(&parent, 30.0, 70.0);
    let n2 = create_node(&parent, 30.0, 30.0);
    let n3 = create_node(&parent, 30.0, 30.0);
    let n4 = create_node(&parent, 30.0, 30.0);
    let n5 = create_node(&parent, 30.0, 30.0);
    let n6 = create_node(&parent, 30.0, 30.0);
    let n7 = create_node(&parent, 100.0, 10.0);

    run_layout(&parent);

    assert_graph_size(&parent, 110.0, 130.0);
    assert_node_position(&n1, 0.0, 0.0);
    assert_node_position(&n2, 40.0, 0.0);
    assert_node_position(&n3, 80.0, 0.0);
    assert_node_position(&n4, 40.0, 40.0);
    assert_node_position(&n5, 80.0, 40.0);
    assert_node_position(&n6, 0.0, 80.0);
    assert_node_position(&n7, 0.0, 120.0);
}

#[test]
fn test_place_block_from_next_row_right() {
    init_rectpacking_options();

    let parent = create_compaction_graph(110.0, 10.0);
    let n1 = create_node(&parent, 30.0, 70.0);
    let n2 = create_node(&parent, 30.0, 30.0);
    let n3 = create_node(&parent, 20.0, 20.0);
    let n4 = create_node(&parent, 30.0, 70.0);

    run_layout(&parent);

    assert_graph_size(&parent, 110.0, 70.0);
    assert_node_position(&n1, 0.0, 0.0);
    assert_node_position(&n2, 40.0, 0.0);
    assert_node_position(&n3, 40.0, 40.0);
    assert_node_position(&n4, 80.0, 0.0);
}

#[test]
fn test_place_block_from_current_row_on_top() {
    init_rectpacking_options();

    let parent = create_compaction_graph(150.0, 10.0);
    let n1 = create_node(&parent, 30.0, 70.0);
    let n2 = create_node(&parent, 30.0, 10.0);
    let n3 = create_node(&parent, 30.0, 10.0);
    let n4 = create_node(&parent, 30.0, 50.0);

    run_layout(&parent);

    assert_graph_size(&parent, 110.0, 70.0);
    assert_node_position(&n1, 0.0, 0.0);
    assert_node_position(&n2, 40.0, 0.0);
    assert_node_position(&n3, 80.0, 0.0);
    assert_node_position(&n4, 40.0, 20.0);
}

fn create_compaction_graph(target_width: f64, spacing: f64) -> ElkNodeRef {
    let parent = ElkGraphUtil::create_graph();
    set_node_property(
        &parent,
        CoreOptions::ALGORITHM,
        RectPackingOptions::ALGORITHM_ID.to_string(),
    );
    set_node_property(
        &parent,
        RectPackingOptions::WIDTH_APPROXIMATION_TARGET_WIDTH,
        target_width,
    );
    set_node_property(
        &parent,
        RectPackingOptions::WIDTH_APPROXIMATION_STRATEGY,
        WidthApproximationStrategy::TargetWidth,
    );
    set_node_property(&parent, CoreOptions::SPACING_NODE_NODE, spacing);
    set_node_property(&parent, CoreOptions::PADDING, ElkPadding::with_any(0.0));
    parent
}

fn create_node(parent: &ElkNodeRef, width: f64, height: f64) -> ElkNodeRef {
    let node = ElkGraphUtil::create_node(Some(parent.clone()));
    set_dimensions(&node, width, height);
    node
}

fn run_layout(parent: &ElkNodeRef) {
    let mut provider = RectPackingLayoutProvider::new();
    let mut monitor = BasicProgressMonitor::new();
    provider.layout(parent, &mut monitor);
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn assert_graph_size(node: &ElkNodeRef, expected_width: f64, expected_height: f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    assert_close(shape.width(), expected_width, "graph width");
    assert_close(shape.height(), expected_height, "graph height");
}

fn assert_node_position(node: &ElkNodeRef, expected_x: f64, expected_y: f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    assert_close(shape.x(), expected_x, "node x");
    assert_close(shape.y(), expected_y, "node y");
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
