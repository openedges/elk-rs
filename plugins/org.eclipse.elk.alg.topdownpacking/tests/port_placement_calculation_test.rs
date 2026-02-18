use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_alg_topdownpacking::org::eclipse::elk::alg::topdownpacking::options::TopdownpackingMetaDataProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutAlgorithmResolver;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::size_constraint::SizeConstraint;
use org_eclipse_elk_core::org::eclipse::elk::core::options::topdown_node_types::TopdownNodeTypes;
use org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, ElkUtil, EnumSet};
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::LayoutConfigurator;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkNodeRef, ElkPortRef};

const EPS: f64 = 1.0e-4;

fn init_topdown_layout() {
    initialize_plain_java_layout();
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&TopdownpackingMetaDataProvider);
}

#[test]
fn test_bottom_up() {
    init_topdown_layout();
    let graph = ElkGraphUtil::create_graph();

    let port_spacing = 10.0;
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_property(
        &node,
        CoreOptions::NODE_SIZE_CONSTRAINTS,
        EnumSet::of(&[SizeConstraint::PortLabels, SizeConstraint::Ports]),
    );
    set_node_property(&node, CoreOptions::SPACING_PORT_PORT, port_spacing);
    set_dimensions(&node, 20.0, 60.0);

    let port1 = ElkGraphUtil::create_port(Some(node.clone()));
    let port2 = ElkGraphUtil::create_port(Some(node.clone()));
    let port3 = ElkGraphUtil::create_port(Some(node.clone()));

    run_layout(&graph);

    assert_close(
        (port_y(&port1) - port_y(&port2)).abs(),
        port_spacing,
        "port spacing 1",
    );
    assert_close(
        (port_y(&port2) - port_y(&port3)).abs(),
        port_spacing,
        "port spacing 2",
    );
}

#[test]
fn test_top_down() {
    init_topdown_layout();
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::TOPDOWN_LAYOUT, true);
    set_node_property(
        &graph,
        CoreOptions::TOPDOWN_NODE_TYPE,
        TopdownNodeTypes::RootNode,
    );

    let port_spacing = 10.0;
    let fixed_node_height = 60.0;
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_property(&node, CoreOptions::TOPDOWN_LAYOUT, true);
    set_node_property(
        &node,
        CoreOptions::TOPDOWN_NODE_TYPE,
        TopdownNodeTypes::HierarchicalNode,
    );
    set_node_property(&node, CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE, true);
    set_node_property(
        &node,
        CoreOptions::NODE_SIZE_CONSTRAINTS,
        EnumSet::of(&[SizeConstraint::PortLabels, SizeConstraint::Ports]),
    );
    set_node_property(&node, CoreOptions::SPACING_PORT_PORT, port_spacing);
    set_dimensions(&node, 20.0, fixed_node_height);

    let port1 = ElkGraphUtil::create_port(Some(node.clone()));
    let port2 = ElkGraphUtil::create_port(Some(node.clone()));
    let port3 = ElkGraphUtil::create_port(Some(node.clone()));

    run_layout(&graph);

    let expected_spacing = fixed_node_height / 4.0;
    assert_close(
        (port_y(&port1) - port_y(&port2)).abs(),
        expected_spacing,
        "port spacing 1",
    );
    assert_close(
        (port_y(&port2) - port_y(&port3)).abs(),
        expected_spacing,
        "port spacing 2",
    );
}

fn run_layout(graph: &ElkNodeRef) {
    let mut configurator = LayoutConfigurator::new();
    let mut resolver = LayoutAlgorithmResolver::new();
    ElkUtil::apply_visitors(graph, &mut [&mut configurator, &mut resolver]);

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout(graph, &mut monitor);
}

fn port_y(port: &ElkPortRef) -> f64 {
    let mut port_mut = port.borrow_mut();
    port_mut.connectable().shape().y()
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
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

fn assert_close(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= EPS,
        "{context} mismatch: actual={actual}, expected={expected}, eps={EPS}"
    );
}
