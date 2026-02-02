use org_eclipse_elk_core::org::eclipse::elk::core::{
    LayoutConfigurator, LayoutConfiguratorClass, NO_OVERWRITE,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const EPSILON: f64 = 0.0001;

#[test]
fn layout_configurator_element_overrides_class() {
    let graph = Graph::new();

    let mut configurator = LayoutConfigurator::new();
    configurator
        .configure_class(LayoutConfiguratorClass::Node)
        .set_property(CoreOptions::SPACING_NODE_NODE, Some(22.0));
    configurator
        .configure_node(&graph.n1)
        .set_property(CoreOptions::SPACING_NODE_NODE, Some(12.0));
    configurator
        .configure_node(&graph.n2)
        .set_property(CoreOptions::SPACING_NODE_NODE, Some(32.0));

    ElkUtil::apply_visitors(&graph.root, &mut [&mut configurator]);

    assert_eq_f64(
        get_node_property(&graph.root, CoreOptions::SPACING_NODE_NODE).unwrap(),
        22.0,
    );
    assert_eq_f64(
        get_node_property(&graph.n1, CoreOptions::SPACING_NODE_NODE).unwrap(),
        12.0,
    );
    assert_eq_f64(
        get_node_property(&graph.n2, CoreOptions::SPACING_NODE_NODE).unwrap(),
        32.0,
    );
}

#[test]
fn layout_configurator_combine_element_and_class() {
    let graph = Graph::new();

    let mut configurator = LayoutConfigurator::new();
    configurator
        .configure_class(LayoutConfiguratorClass::Node)
        .set_property(CoreOptions::SPACING_NODE_NODE, Some(42.0));
    configurator
        .configure_node(&graph.n1)
        .set_property(CoreOptions::SPACING_EDGE_NODE, Some(42.0));

    ElkUtil::apply_visitors(&graph.root, &mut [&mut configurator]);

    assert_eq_f64(
        get_node_property(&graph.n1, CoreOptions::SPACING_NODE_NODE).unwrap(),
        42.0,
    );
    assert_eq_f64(
        get_node_property(&graph.n1, CoreOptions::SPACING_EDGE_NODE).unwrap(),
        42.0,
    );
}

#[test]
fn layout_configurator_combine_multiple_class() {
    let graph = Graph::new();

    let mut configurator = LayoutConfigurator::new();
    configurator
        .configure_class(LayoutConfiguratorClass::Node)
        .set_property(CoreOptions::SPACING_NODE_NODE, Some(42.0));
    configurator
        .configure_class(LayoutConfiguratorClass::GraphElement)
        .set_property(CoreOptions::SPACING_EDGE_NODE, Some(42.0));

    ElkUtil::apply_visitors(&graph.root, &mut [&mut configurator]);

    assert_eq_f64(
        get_node_property(&graph.n1, CoreOptions::SPACING_NODE_NODE).unwrap(),
        42.0,
    );
    assert_eq_f64(
        get_node_property(&graph.n1, CoreOptions::SPACING_EDGE_NODE).unwrap(),
        42.0,
    );
}

#[test]
fn layout_configurator_overrides_existing_options() {
    let graph = Graph::new();
    set_node_property(&graph.root, CoreOptions::SPACING_NODE_NODE, 13.0);

    let mut configurator = LayoutConfigurator::new();
    configurator
        .configure_class(LayoutConfiguratorClass::Node)
        .set_property(CoreOptions::SPACING_NODE_NODE, Some(42.0));

    ElkUtil::apply_visitors(&graph.root, &mut [&mut configurator]);

    assert_eq_f64(
        get_node_property(&graph.root, CoreOptions::SPACING_NODE_NODE).unwrap(),
        42.0,
    );
}

#[test]
fn layout_configurator_prevent_overriding_existing_options() {
    let graph = Graph::new();
    set_node_property(&graph.root, CoreOptions::SPACING_NODE_NODE, 13.0);

    let mut configurator = LayoutConfigurator::new();
    configurator
        .configure_class(LayoutConfiguratorClass::Node)
        .set_property(CoreOptions::SPACING_NODE_NODE, Some(42.0));
    configurator.add_filter(NO_OVERWRITE.clone());

    ElkUtil::apply_visitors(&graph.root, &mut [&mut configurator]);

    assert_eq_f64(
        get_node_property(&graph.root, CoreOptions::SPACING_NODE_NODE).unwrap(),
        13.0,
    );
}

struct Graph {
    root: ElkNodeRef,
    n1: ElkNodeRef,
    n2: ElkNodeRef,
}

impl Graph {
    fn new() -> Self {
        let root = ElkGraphUtil::create_graph();
        let n1 = ElkGraphUtil::create_node(Some(root.clone()));
        let n2 = ElkGraphUtil::create_node(Some(root.clone()));
        Graph { root, n1, n2 }
    }
}

fn set_node_property(node: &ElkNodeRef, property: &'static std::sync::LazyLock<org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<f64>>, value: f64) {
    with_node_properties_mut(node, |props| {
        props.set_property(property, Some(value));
    });
}

fn get_node_property(
    node: &ElkNodeRef,
    property: &'static std::sync::LazyLock<org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property<f64>>,
) -> Option<f64> {
    with_node_properties_mut(node, |props| props.get_property(property))
}

fn with_node_properties_mut<R>(node: &ElkNodeRef, f: impl FnOnce(&mut MapPropertyHolder) -> R) -> R {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    f(props)
}

fn assert_eq_f64(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= EPSILON);
}
