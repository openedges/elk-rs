use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::persistence::{
    ElkGraphResourceFactory, ElkGraphXMIHelper, ElkGraphXMISave,
};

#[test]
fn resource_factory_defaults_to_utf8() {
    let factory = ElkGraphResourceFactory::new();
    let resource = factory.create_resource();
    assert_eq!(resource.encoding(), "utf-8");
}

#[test]
fn xmi_helper_creates_proxy_value() {
    let helper = ElkGraphXMIHelper::new();
    let proxy = helper.create_property_value("123");
    assert_eq!(proxy.value(), "123");
}

#[test]
fn xmi_save_filters_unparsable_properties() {
    LayoutMetaDataService::get_instance();

    let save = ElkGraphXMISave::new();
    assert!(save.should_serialize_property(CoreOptions::SPACING_NODE_NODE.id()));
    assert!(!save.should_serialize_property("unknown.option"));
}
