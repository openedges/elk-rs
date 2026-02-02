use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::internal::LayoutOptionProxy;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;

#[test]
fn layout_option_proxy_resolves_value() {
    LayoutMetaDataService::get_instance();

    let mut holder = MapPropertyHolder::new();
    LayoutOptionProxy::set_proxy_value(
        &mut holder,
        CoreOptions::SPACING_NODE_NODE.id(),
        "42",
    );

    let value = holder
        .get_property(CoreOptions::SPACING_NODE_NODE)
        .expect("proxy should resolve");
    assert!((value - 42.0).abs() < f64::EPSILON);
}
