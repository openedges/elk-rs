use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const TEST_PROPERTY_ID: &str = "org.eclipse.elk.alg.common.test";
const TEST_PROPERTY_VALUE: &str = "Test";

#[test]
fn configurator_test() {
    let test_property = Property::with_default(TEST_PROPERTY_ID, String::new());
    let graph = ElkGraphUtil::create_graph();

    configure_stuff(&graph, &test_property);

    let actual = node_property(&graph, &test_property).unwrap_or_default();
    assert_eq!(actual, TEST_PROPERTY_VALUE);
}

fn configure_stuff(node: &ElkNodeRef, property: &Property<String>) {
    set_node_property(node, property, TEST_PROPERTY_VALUE.to_string());
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
