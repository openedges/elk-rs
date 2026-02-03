use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    LayoutMetaDataService, LayoutOptionTarget, LayoutOptionVisibility,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;

#[test]
fn label_manager_option_registered() {
    let option = LayoutMetaDataService::get_instance()
        .get_option_data(CoreOptions::LABEL_MANAGER.id())
        .expect("label manager option");
    assert!(option.targets().contains(&LayoutOptionTarget::Parents));
    assert!(option.targets().contains(&LayoutOptionTarget::Labels));
    assert_eq!(option.visibility(), LayoutOptionVisibility::Hidden);
}
