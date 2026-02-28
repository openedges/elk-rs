use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::options::{
    RectPackingMetaDataProvider, RectPackingOptions,
};
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::p2packing::PackingStrategy;
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

#[test]
fn test_decrease_approximated_width() {
    init_rectpacking_options();

    let parent = ElkGraphUtil::create_graph();
    set_node_property(
        &parent,
        CoreOptions::ALGORITHM,
        RectPackingOptions::ALGORITHM_ID.to_string(),
    );
    set_node_property(&parent, CoreOptions::SPACING_NODE_NODE, 0.0_f64);
    set_node_property(&parent, CoreOptions::PADDING, ElkPadding::with_any(0.0));

    let n1 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n2 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n3 = ElkGraphUtil::create_node(Some(parent.clone()));
    let n4 = ElkGraphUtil::create_node(Some(parent.clone()));
    set_dimensions(&n1, 30.0, 30.0);
    set_dimensions(&n2, 10.0, 10.0);
    set_dimensions(&n3, 30.0, 30.0);
    set_dimensions(&n4, 40.0, 10.0);

    set_node_property(
        &parent,
        RectPackingOptions::PACKING_STRATEGY,
        PackingStrategy::None,
    );
    run_layout(&parent);
    assert_close(node_width(&parent), 60.0, "width with approximation only");

    set_node_property(
        &parent,
        RectPackingOptions::PACKING_STRATEGY,
        PackingStrategy::Compaction,
    );
    run_layout(&parent);
    assert_close(node_width(&parent), 40.0, "width with compaction");
}

fn init_rectpacking_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&RectPackingMetaDataProvider);
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

fn node_width(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().width()
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
