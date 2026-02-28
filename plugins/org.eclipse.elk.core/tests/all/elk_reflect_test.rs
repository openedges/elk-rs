use std::sync::OnceLock;

use org_eclipse_elk_alg_disco::org::eclipse::elk::alg::disco::options::DisCoMetaDataProvider;
use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::options::{
    ForceMetaDataProvider, StressMetaDataProvider,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredMetaDataProvider;
use org_eclipse_elk_alg_mrtree::org::eclipse::elk::alg::mrtree::options::MrTreeMetaDataProvider;
use org_eclipse_elk_alg_radial::org::eclipse::elk::alg::radial::options::RadialMetaDataProvider;
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::options::RectPackingMetaDataProvider;
use org_eclipse_elk_alg_spore::org::eclipse::elk::alg::spore::options::SporeMetaDataProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

fn init_algorithm_metadata() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&DisCoMetaDataProvider);
        service.register_layout_meta_data_provider(&ForceMetaDataProvider);
        service.register_layout_meta_data_provider(&StressMetaDataProvider);
        service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
        service.register_layout_meta_data_provider(&MrTreeMetaDataProvider);
        service.register_layout_meta_data_provider(&RadialMetaDataProvider);
        service.register_layout_meta_data_provider(&RectPackingMetaDataProvider);
        service.register_layout_meta_data_provider(&SporeMetaDataProvider);
    });
}

#[test]
fn layout_option_defaults_are_cloneable() {
    init_algorithm_metadata();

    let service = LayoutMetaDataService::get_instance();
    for option in service.get_option_data_list() {
        let Some(default_value) = option.default_value() else {
            continue;
        };

        let cloned = ElkReflect::clone_any(default_value.as_ref()).unwrap_or_else(|| {
            panic!(
                "ElkReflect clone missing for default of option '{}'",
                option.id()
            )
        });

        assert_eq!(
            cloned.as_ref().type_id(),
            default_value.as_ref().type_id(),
            "ElkReflect clone type mismatch for option '{}'",
            option.id()
        );
    }
}
