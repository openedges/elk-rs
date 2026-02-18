use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_core::org::eclipse::elk::core::{LayoutConfigurator, LayoutConfiguratorClass};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

const TEST_PROPERTY_ID: &str = "org.eclipse.elk.alg.common.test";
const TEST_PROPERTY_VALUE: &str = "Test";

#[test]
fn configurator_provider_test() {
    let test_property = Property::with_default(TEST_PROPERTY_ID, String::new());
    let graph = ElkGraphUtil::create_graph();

    let mut configurator = LayoutConfigurator::new();
    configurator
        .configure_class(LayoutConfiguratorClass::Node)
        .set_property(&test_property, Some(TEST_PROPERTY_VALUE.to_string()));

    ElkUtil::apply_visitors(&graph, &mut [&mut configurator]);

    let actual = node_property(&graph, &test_property).unwrap_or_default();
    assert_eq!(actual, TEST_PROPERTY_VALUE);
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
