use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredMetaDataProvider;
use org_eclipse_elk_alg_topdownpacking::org::eclipse::elk::alg::topdownpacking::options::TopdownpackingMetaDataProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutAlgorithmData, LayoutMetaDataRegistry, LayoutMetaDataService,
};
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::topdown_node_types::TopdownNodeTypes;
use org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    AlgorithmFactory, BasicProgressMonitor, InstancePool,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const EPS: f64 = 1.0e-5;

fn init_topdown_layout() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredAlgorithmProvider);
    service.register_layout_meta_data_provider(&TopdownpackingMetaDataProvider);
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

struct LayeredAlgorithmProvider;

impl ILayoutMetaDataProvider for LayeredAlgorithmProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        let factory = AlgorithmFactory::new(|| Box::new(LayeredLayoutProvider::new()));
        let pool = InstancePool::new(Box::new(factory));
        let mut data =
            LayoutAlgorithmData::new("org.eclipse.elk.layered").with_provider_pool(Arc::new(pool));
        data.set_category_id(Some("org.eclipse.elk.layered"))
            .set_defining_bundle_id(Some("org.eclipse.elk.alg.layered"))
            .set_preview_image_path(Some("images/layered_layout.png"));
        registry.register_algorithm(data);
    }
}

#[test]
fn test_two_level_layout_horizontal_scaling() {
    init_topdown_layout();
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::TOPDOWN_LAYOUT, true);
    set_node_property(&graph, CoreOptions::TOPDOWN_NODE_TYPE, TopdownNodeTypes::RootNode);

    let toplevel = ElkGraphUtil::create_node(Some(graph.clone()));
    configure_toplevel_for_scaling(&toplevel, 20.0, 0.4, 1.0);
    add_fixed_child(&toplevel, 0.0, 0.0, 30.0, 30.0);
    add_fixed_child(&toplevel, 0.0, 40.0, 30.0, 30.0);

    run_recursive_layout(&graph);

    let scale = node_property(&toplevel, CoreOptions::TOPDOWN_SCALE_FACTOR)
        .expect("topdown scale factor should be set");
    assert_close(scale, 20.0 / 30.0, "horizontal scale");
}

#[test]
fn test_two_level_layout_vertical_scaling() {
    init_topdown_layout();
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::TOPDOWN_LAYOUT, true);
    set_node_property(&graph, CoreOptions::TOPDOWN_NODE_TYPE, TopdownNodeTypes::RootNode);

    let toplevel = ElkGraphUtil::create_node(Some(graph.clone()));
    configure_toplevel_for_scaling(&toplevel, 40.0, 1.33333, 1.0);
    add_fixed_child(&toplevel, 0.0, 0.0, 30.0, 30.0);
    add_fixed_child(&toplevel, 0.0, 40.0, 30.0, 30.0);

    run_recursive_layout(&graph);

    let scale = node_property(&toplevel, CoreOptions::TOPDOWN_SCALE_FACTOR)
        .expect("topdown scale factor should be set");
    assert_close(scale, 30.0 / 70.0, "vertical scale");
}

#[test]
fn test_scale_cap() {
    init_topdown_layout();
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::TOPDOWN_LAYOUT, true);
    set_node_property(&graph, CoreOptions::TOPDOWN_NODE_TYPE, TopdownNodeTypes::RootNode);

    let toplevel = ElkGraphUtil::create_node(Some(graph.clone()));
    configure_toplevel_for_scaling(&toplevel, 300.0, 1.0, 100.0);
    add_fixed_child(&toplevel, 0.0, 0.0, 30.0, 30.0);
    add_fixed_child(&toplevel, 0.0, 40.0, 30.0, 30.0);

    run_recursive_layout(&graph);

    let scale = node_property(&toplevel, CoreOptions::TOPDOWN_SCALE_FACTOR)
        .expect("topdown scale factor should be set");
    assert_close(scale, 300.0 / 70.0, "scale cap not bounding");
}

#[test]
fn test_scale_cap_bounded() {
    init_topdown_layout();
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::TOPDOWN_LAYOUT, true);
    set_node_property(&graph, CoreOptions::TOPDOWN_NODE_TYPE, TopdownNodeTypes::RootNode);

    let toplevel = ElkGraphUtil::create_node(Some(graph.clone()));
    configure_toplevel_for_scaling(&toplevel, 300.0, 1.0, 3.0);
    add_fixed_child(&toplevel, 0.0, 0.0, 30.0, 30.0);
    add_fixed_child(&toplevel, 0.0, 40.0, 30.0, 30.0);

    run_recursive_layout(&graph);

    let scale = node_property(&toplevel, CoreOptions::TOPDOWN_SCALE_FACTOR)
        .expect("topdown scale factor should be set");
    assert_close(scale, 3.0, "scale cap bounding");
}

#[test]
fn test_child_dimension_calculation() {
    init_topdown_layout();
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::TOPDOWN_LAYOUT, true);
    set_node_property(&graph, CoreOptions::TOPDOWN_NODE_TYPE, TopdownNodeTypes::RootNode);

    let toplevel = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_property(&toplevel, CoreOptions::TOPDOWN_LAYOUT, true);
    set_node_property(
        &toplevel,
        CoreOptions::TOPDOWN_NODE_TYPE,
        TopdownNodeTypes::HierarchicalNode,
    );
    set_node_property(&toplevel, CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE, true);
    set_node_property(
        &toplevel,
        CoreOptions::ALGORITHM,
        "org.eclipse.elk.layered".to_string(),
    );
    set_node_property(
        &toplevel,
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH,
        20.0_f64,
    );
    set_node_property(
        &toplevel,
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO,
        1.0_f64,
    );
    let padding = ElkPadding::with_any(10.0);
    set_node_property(&toplevel, CoreOptions::PADDING, padding.clone());

    let child = ElkGraphUtil::create_node(Some(toplevel.clone()));
    set_node_property(&child, CoreOptions::TOPDOWN_LAYOUT, true);
    set_node_property(
        &child,
        CoreOptions::TOPDOWN_NODE_TYPE,
        TopdownNodeTypes::HierarchicalNode,
    );
    set_node_property(&child, CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE, true);
    set_node_property(&child, CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH, 20.0_f64);
    set_node_property(
        &child,
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO,
        1.0_f64,
    );
    set_location(&child, 0.0, 0.0);

    run_recursive_layout(&graph);

    let (width, height) = node_dimensions(&toplevel);
    assert_close(width, 20.0 + padding.left + padding.right, "child dimension width");
    assert_close(height, 20.0 + padding.top + padding.bottom, "child dimension height");
}

fn configure_toplevel_for_scaling(
    node: &ElkNodeRef,
    width: f64,
    aspect_ratio: f64,
    scale_cap: f64,
) {
    set_node_property(node, CoreOptions::TOPDOWN_LAYOUT, true);
    set_node_property(
        node,
        CoreOptions::TOPDOWN_NODE_TYPE,
        TopdownNodeTypes::HierarchicalNode,
    );
    set_node_property(node, CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE, true);
    set_node_property(node, CoreOptions::TOPDOWN_SCALE_CAP, scale_cap);
    set_node_property(node, CoreOptions::ALGORITHM, "org.eclipse.elk.fixed".to_string());
    set_node_property(node, CoreOptions::PADDING, ElkPadding::new());
    set_node_property(node, CoreOptions::SPACING_NODE_NODE, 0.0_f64);
    set_node_property(node, CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH, width);
    set_node_property(
        node,
        CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO,
        aspect_ratio,
    );
}

fn add_fixed_child(
    parent: &ElkNodeRef,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> ElkNodeRef {
    let child = ElkGraphUtil::create_node(Some(parent.clone()));
    set_node_property(&child, CoreOptions::TOPDOWN_LAYOUT, true);
    set_node_property(
        &child,
        CoreOptions::TOPDOWN_NODE_TYPE,
        TopdownNodeTypes::HierarchicalNode,
    );
    set_node_property(&child, CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE, true);
    set_location(&child, x, y);
    set_dimensions(&child, width, height);
    child
}

fn run_recursive_layout(graph: &ElkNodeRef) {
    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout(graph, &mut monitor);
}

fn node_dimensions(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn set_location(node: &ElkNodeRef, x: f64, y: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_location(x, y);
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

fn node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
) -> Option<T> {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
}
