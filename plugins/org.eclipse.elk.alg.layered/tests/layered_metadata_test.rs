use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions, LayerUnzippingStrategy, WrappingStrategy,
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

#[test]
fn wrapping_strategy_defaults() {
    init_layered_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::WRAPPING_STRATEGY.id())
        .expect("wrapping strategy option");

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<WrappingStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, WrappingStrategy::Off);
}

#[test]
fn layer_unzipping_defaults() {
    init_layered_options();

    let strategy = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::LAYER_UNZIPPING_STRATEGY.id())
        .expect("layer unzipping strategy option");
    let default = strategy
        .default_value()
        .and_then(|value| value.downcast::<LayerUnzippingStrategy>().ok())
        .expect("default value");
    assert_eq!(*default, LayerUnzippingStrategy::None);

    let split = LayoutMetaDataService::get_instance()
        .get_option_data(LayeredOptions::LAYER_UNZIPPING_LAYER_SPLIT.id())
        .expect("layer split option");
    let default = split
        .default_value()
        .and_then(|value| value.downcast::<i32>().ok())
        .expect("default value");
    assert_eq!(*default, 2);

    let lower = split
        .lower_bound()
        .and_then(|value| value.downcast::<i32>().ok())
        .expect("lower bound");
    assert_eq!(*lower, 1);
}
