use org_eclipse_elk_alg_mrtree::org::eclipse::elk::alg::mrtree::options::{
    MrTreeMetaDataProvider, MrTreeOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    LayoutAlgorithmResolver, LayoutMetaDataService,
};
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, ElkUtil};
use org_eclipse_elk_core::org::eclipse::elk::core::LayoutConfigurator;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};

const DOUBLE_EQ_EPSILON: f64 = 1.0e-4;
const CASES: [(f64, f64, f64, f64, f64, f64, f64); 3] = [
    (20.0, 20.0, 10.0, 10.0, 10.0, 10.0, 10.0),
    (15.0, 30.0, 7.0, 9.0, 11.0, 13.0, 15.0),
    (25.0, 15.0, 0.0, 0.0, 0.0, 0.0, 20.0),
];

fn init_mrtree_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&MrTreeMetaDataProvider);
}

#[test]
fn graph_size_calculation_test() {
    init_mrtree_options();

    for (
        node_width,
        node_height,
        padding_left,
        padding_right,
        padding_top,
        padding_bottom,
        node_node_spacing,
    ) in CASES
    {
        let graph = ElkGraphUtil::create_graph();
        set_node_property(
            &graph,
            CoreOptions::ALGORITHM,
            MrTreeOptions::ALGORITHM_ID.to_string(),
        );
        set_node_property(
            &graph,
            CoreOptions::PADDING,
            ElkPadding::with_values(padding_top, padding_right, padding_bottom, padding_left),
        );
        set_node_property(&graph, CoreOptions::SPACING_NODE_NODE, node_node_spacing);

        let n1 = create_node(&graph, node_width, node_height);
        let n2 = create_node(&graph, node_width, node_height);
        let n3 = create_node(&graph, node_width, node_height);

        ElkGraphUtil::create_simple_edge(
            ElkConnectableShapeRef::Node(n1.clone()),
            ElkConnectableShapeRef::Node(n2),
        );
        ElkGraphUtil::create_simple_edge(
            ElkConnectableShapeRef::Node(n1),
            ElkConnectableShapeRef::Node(n3),
        );

        run_recursive_layout(&graph);

        let (graph_width, graph_height) = node_dimensions(&graph);
        let expected_width =
            padding_left + node_width + node_node_spacing + node_width + padding_right;
        let expected_height =
            padding_top + node_height + node_node_spacing + node_height + padding_bottom;

        assert_close(graph_width, expected_width, "graph width");
        assert_close(graph_height, expected_height, "graph height");
    }
}

#[test]
fn components_graph_size_calculation_test() {
    init_mrtree_options();

    for (node_width, node_height, _, _, _, _, _) in CASES {
        let graph = ElkGraphUtil::create_graph();
        set_node_property(
            &graph,
            CoreOptions::ALGORITHM,
            MrTreeOptions::ALGORITHM_ID.to_string(),
        );
        set_node_property(&graph, CoreOptions::PADDING, ElkPadding::with_any(0.0));
        set_node_property(&graph, CoreOptions::SPACING_NODE_NODE, 0.0_f64);
        // Force horizontal component packing to keep the expectation stable.
        set_node_property(&graph, CoreOptions::ASPECT_RATIO, 1000.0_f64);

        create_node(&graph, node_width, node_height);
        create_node(&graph, node_width, node_height);

        run_recursive_layout(&graph);

        let (graph_width, graph_height) = node_dimensions(&graph);
        assert_close(
            graph_width,
            node_width + node_width,
            "components graph width",
        );
        assert_close(graph_height, node_height, "components graph height");
    }
}

fn create_node(parent: &ElkNodeRef, width: f64, height: f64) -> ElkNodeRef {
    let node = ElkGraphUtil::create_node(Some(parent.clone()));
    {
        let mut node_mut = node.borrow_mut();
        node_mut.connectable().shape().set_dimensions(width, height);
    }
    node
}

fn run_recursive_layout(graph: &ElkNodeRef) {
    let mut configurator = LayoutConfigurator::new();
    let mut resolver = LayoutAlgorithmResolver::new();
    ElkUtil::apply_visitors(graph, &mut [&mut configurator, &mut resolver]);

    let mut engine = RecursiveGraphLayoutEngine::new();
    let mut monitor = BasicProgressMonitor::new();
    engine.layout(graph, &mut monitor);
}

fn node_dimensions(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
}

fn assert_close(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= DOUBLE_EQ_EPSILON,
        "{context} mismatch: actual={actual}, expected={expected}, eps={DOUBLE_EQ_EPSILON}"
    );
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &'static std::sync::LazyLock<Property<T>>,
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
