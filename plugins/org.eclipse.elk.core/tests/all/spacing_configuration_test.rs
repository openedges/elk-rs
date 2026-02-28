use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_spacings::ElkCoreSpacingsBuilder;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    ElkSpacings, ElkUtil, IGraphElementVisitor,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkGraphElementRef, ElkNodeRef,
};

const DOUBLE_EQ_EPSILON: f64 = 10e-5;

#[test]
fn test_create_visitor() {
    init_metadata();
    let node = ElkGraphUtil::create_node(None);
    let mut visitor = ElkSpacings::with_base_value(3.0).to_visitor();
    visitor.visit(&ElkGraphElementRef::Node(node.clone()));

    let value = node_property(&node, ElkCoreSpacingsBuilder::BASE_SPACING_OPTION)
        .expect("Expected base spacing to be set.");
    assert_close(3.0, value);
}

#[test]
fn test_apply_to() {
    init_metadata();
    let node = ElkGraphUtil::create_node(None);
    ElkSpacings::with_base_value(3.0).apply(&ElkGraphElementRef::Node(node.clone()));

    let value = node_property(&node, ElkCoreSpacingsBuilder::BASE_SPACING_OPTION)
        .expect("Expected base spacing to be set.");
    assert_close(3.0, value);
}

#[test]
#[should_panic]
fn test_spacing_with_negative_factor() {
    init_metadata();
    let _graph = create_and_configure_test_graph(
        ElkSpacings::with_base_value(33.0)
            .with_factor(CoreOptions::SPACING_EDGE_EDGE, -1.0)
            .to_visitor(),
    );
}

#[test]
#[should_panic]
fn test_spacing_with_negative_value() {
    init_metadata();
    let _graph = create_and_configure_test_graph(
        ElkSpacings::with_base_value(33.0)
            .with_value(CoreOptions::SPACING_EDGE_EDGE, -1.0)
            .to_visitor(),
    );
}

#[test]
fn test_default_factors() {
    init_metadata();
    let builder = ElkSpacings::with_base_value(33.0);
    let base_default = ElkCoreSpacingsBuilder::BASE_SPACING_OPTION
        .get_default()
        .unwrap_or(0.0);

    for option in spacing_options() {
        let option_ref: &Property<f64> = option;
        let factor = find_factor(&builder, option_ref)
            .unwrap_or_else(|| panic!("Missing factor for {}", option_ref.id()));
        let expected = option_ref.get_default().unwrap_or(0.0) / base_default;
        assert_close(expected, factor);
    }
}

#[test]
fn test_spacing_with_factor() {
    init_metadata();
    for option in spacing_options() {
        let option_ref: &Property<f64> = option;
        let graph = create_and_configure_test_graph(
            ElkSpacings::with_base_value(33.0)
                .with_factor(option_ref, 2.0)
                .to_visitor(),
        );
        check_option_value(&graph, option_ref, 66.0);
    }
}

#[test]
fn test_spacing_with_value() {
    init_metadata();
    for option in spacing_options() {
        let option_ref: &Property<f64> = option;
        let graph = create_and_configure_test_graph(
            ElkSpacings::with_base_value(33.0)
                .with_value(option_ref, 24.0)
                .to_visitor(),
        );
        check_option_value(&graph, option_ref, 24.0);
    }
}

#[test]
fn test_overwrite_if_requested() {
    init_metadata();
    for option in spacing_options() {
        let option_ref: &Property<f64> = option;
        let graph = create_simple_graph();
        let (first_child, second_child) = child_nodes(&graph);

        set_node_property(&first_child, option_ref, 22.0);

        let mut visitor = ElkSpacings::with_base_value(1.0)
            .with_value(option_ref, 3.0)
            .with_overwrite(true)
            .to_visitor();
        ElkUtil::apply_visitors(&graph, &mut [&mut *visitor]);

        let first_value = node_property(&first_child, option_ref).unwrap_or(0.0);
        let second_value = node_property(&second_child, option_ref).unwrap_or(0.0);
        assert_close(3.0, first_value);
        assert_close(3.0, second_value);
    }
}

#[test]
fn test_dont_overwrite_if_not_requested() {
    init_metadata();
    for option in spacing_options() {
        let option_ref: &Property<f64> = option;
        let graph = create_simple_graph();
        let (first_child, second_child) = child_nodes(&graph);

        set_node_property(&first_child, option_ref, 22.0);

        let mut visitor = ElkSpacings::with_base_value(1.0)
            .with_value(option_ref, 3.0)
            .with_overwrite(false)
            .to_visitor();
        ElkUtil::apply_visitors(&graph, &mut [&mut *visitor]);

        let first_value = node_property(&first_child, option_ref).unwrap_or(0.0);
        let second_value = node_property(&second_child, option_ref).unwrap_or(0.0);
        assert_close(22.0, first_value);
        assert_close(3.0, second_value);
    }
}

#[test]
#[should_panic]
fn test_invalid_option_with_factor_panics() {
    init_metadata();
    let _graph = create_and_configure_test_graph(
        ElkSpacings::with_base_value(33.0)
            .with_factor(ElkCoreSpacingsBuilder::BASE_SPACING_OPTION, 2.0)
            .to_visitor(),
    );
}

#[test]
#[should_panic]
fn test_invalid_option_with_value_panics() {
    init_metadata();
    let _graph = create_and_configure_test_graph(
        ElkSpacings::with_base_value(33.0)
            .with_value(CoreOptions::ASPECT_RATIO, 24.0)
            .to_visitor(),
    );
}

fn spacing_options() -> [&'static std::sync::LazyLock<Property<f64>>; 10] {
    [
        CoreOptions::SPACING_COMPONENT_COMPONENT,
        CoreOptions::SPACING_EDGE_EDGE,
        CoreOptions::SPACING_EDGE_LABEL,
        CoreOptions::SPACING_EDGE_NODE,
        CoreOptions::SPACING_LABEL_LABEL,
        CoreOptions::SPACING_LABEL_NODE,
        CoreOptions::SPACING_LABEL_PORT_HORIZONTAL,
        CoreOptions::SPACING_LABEL_PORT_VERTICAL,
        CoreOptions::SPACING_NODE_SELF_LOOP,
        CoreOptions::SPACING_PORT_PORT,
    ]
}

fn init_metadata() {
    LayoutMetaDataService::get_instance();
}

fn create_and_configure_test_graph(mut visitor: Box<dyn IGraphElementVisitor>) -> ElkNodeRef {
    let graph = create_simple_graph();
    ElkUtil::apply_visitors(&graph, &mut [&mut *visitor]);
    graph
}

fn check_option_value(graph: &ElkNodeRef, option: &Property<f64>, value: f64) {
    for node in collect_nodes(graph) {
        let stored = node_property(&node, option).unwrap_or(0.0);
        assert_close(value, stored);
    }
}

fn create_simple_graph() -> ElkNodeRef {
    let graph = ElkGraphUtil::create_graph();
    let node1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let node2 = ElkGraphUtil::create_node(Some(graph.clone()));

    set_dimensions(&node1, 30.0, 30.0);
    set_dimensions(&node2, 30.0, 30.0);

    ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(node1.clone()),
        ElkConnectableShapeRef::Node(node2.clone()),
    );

    graph
}

fn child_nodes(graph: &ElkNodeRef) -> (ElkNodeRef, ElkNodeRef) {
    let children: Vec<ElkNodeRef> = {
        let mut graph_mut = graph.borrow_mut();
        graph_mut.children().iter().cloned().collect()
    };
    (children[0].clone(), children[1].clone())
}

fn collect_nodes(graph: &ElkNodeRef) -> Vec<ElkNodeRef> {
    let mut stack = vec![graph.clone()];
    let mut nodes = Vec::new();
    while let Some(node) = stack.pop() {
        let children: Vec<ElkNodeRef> = {
            let mut node_mut = node.borrow_mut();
            node_mut.children().iter().cloned().collect()
        };
        nodes.push(node.clone());
        for child in children {
            stack.push(child);
        }
    }
    nodes
}

fn node_property(node: &ElkNodeRef, option: &Property<f64>) -> Option<f64> {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(option)
}

fn set_node_property(node: &ElkNodeRef, option: &Property<f64>, value: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(option, Some(value));
}

fn set_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn find_factor(builder: &ElkCoreSpacingsBuilder, option: &Property<f64>) -> Option<f64> {
    builder
        .factors()
        .iter()
        .find(|entry| entry.property.id() == option.id())
        .map(|entry| entry.factor)
}

fn assert_close(expected: f64, actual: f64) {
    assert!(
        (expected - actual).abs() <= DOUBLE_EQ_EPSILON,
        "Expected {expected}, got {actual}"
    );
}
