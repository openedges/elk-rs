use org_eclipse_elk_alg_libavoid::org::eclipse::elk::alg::libavoid::options::{
    LibavoidMetaDataProvider, LibavoidOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    LayoutMetaDataService, LayoutOptionTarget,
};

fn init_libavoid_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LibavoidMetaDataProvider);
}

#[test]
fn libavoid_algorithm_registered() {
    init_libavoid_options();

    let algo = LayoutMetaDataService::get_instance()
        .get_algorithm_data(LibavoidOptions::ALGORITHM_ID)
        .expect("libavoid algorithm");
    assert_eq!(algo.name(), "Libavoid");
    assert_eq!(algo.category_id(), Some("org.eclipse.elk.alg.libavoid.edge"));
}

#[test]
fn segment_penalty_default() {
    init_libavoid_options();

    let option = LayoutMetaDataService::get_instance()
        .get_option_data(LibavoidOptions::SEGMENT_PENALTY.id())
        .expect("segment penalty option");
    assert!(option.targets().contains(&LayoutOptionTarget::Parents));

    let default = option
        .default_value()
        .and_then(|value| value.downcast::<f64>().ok())
        .expect("default segment penalty");
    assert_eq!(*default, 10.0);
}
