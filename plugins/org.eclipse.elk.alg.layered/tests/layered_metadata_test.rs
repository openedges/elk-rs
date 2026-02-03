use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    LayoutMetaDataService, LayoutOptionTarget, LayoutOptionVisibility,
};

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

#[test]
fn spacing_base_value_metadata() {
    init_layered_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::SPACING_BASE_VALUE.id())
        .expect("spacing base value option");

    assert_eq!(option.group(), "spacing");
    assert!(option.targets().contains(&LayoutOptionTarget::Parents));
    assert!(option.default_value().is_none());

    let lower = option
        .lower_bound()
        .and_then(|value| value.downcast::<f64>().ok())
        .expect("lower bound");
    assert!((*lower - 0.0).abs() < f64::EPSILON);
}

#[test]
fn priority_direction_metadata() {
    init_layered_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::PRIORITY_DIRECTION.id())
        .expect("priority direction option");

    assert_eq!(option.group(), "priority");
    assert!(option.targets().contains(&LayoutOptionTarget::Edges));
    assert_eq!(option.visibility(), LayoutOptionVisibility::Advanced);

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<i32>().ok())
        .expect("default value");
    assert_eq!(*default, 0);
}
