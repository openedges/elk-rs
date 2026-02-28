use org_eclipse_elk_alg_radial::org::eclipse::elk::alg::radial::options::{
    AnnulusWedgeCriteria, RadialMetaDataProvider, RadialOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    LayoutMetaDataService, LayoutOptionTarget, LayoutOptionVisibility,
};

fn init_radial_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&RadialMetaDataProvider);
}

#[test]
fn radial_algorithm_registered() {
    init_radial_options();

    let algo = LayoutMetaDataService::get_instance()
        .get_algorithm_data(RadialOptions::ALGORITHM_ID)
        .expect("radial algorithm");
    assert_eq!(algo.name(), "ELK Radial");
}

#[test]
fn wedge_criteria_metadata() {
    init_radial_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(RadialOptions::WEDGE_CRITERIA.id())
        .expect("wedge criteria option");
    assert!(option.targets().contains(&LayoutOptionTarget::Parents));
    assert_eq!(option.visibility(), LayoutOptionVisibility::Visible);

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<AnnulusWedgeCriteria>().ok())
        .expect("default wedge criteria");
    assert_eq!(*default, AnnulusWedgeCriteria::NodeSize);
}

#[test]
fn order_id_targets_nodes() {
    init_radial_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(RadialOptions::ORDER_ID.id())
        .expect("order id option");
    assert!(option.targets().contains(&LayoutOptionTarget::Nodes));
}
