use std::sync::OnceLock;

use org_eclipse_elk_alg_disco::org::eclipse::elk::alg::disco::options::{
    DisCoMetaDataProvider, DisCoOptions,
};
use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::options::{
    ForceMetaDataProvider, ForceOptions, StressMetaDataProvider, StressOptions,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_alg_mrtree::org::eclipse::elk::alg::mrtree::options::{
    MrTreeMetaDataProvider, MrTreeOptions,
};
use org_eclipse_elk_alg_radial::org::eclipse::elk::alg::radial::options::{
    RadialMetaDataProvider, RadialOptions,
};
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::options::{
    RectPackingMetaDataProvider, RectPackingOptions,
};
use org_eclipse_elk_alg_spore::org::eclipse::elk::alg::spore::options::{
    SporeCompactionOptions, SporeMetaDataProvider, SporeOverlapRemovalOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    BoxLayouterOptions, FixedLayouterOptions, RandomLayouterOptions,
};

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
fn plain_java_initialization_registers_expected_algorithms() {
    init_algorithm_metadata();

    let service = LayoutMetaDataService::get_instance();
    let algorithm_ids = [
        BoxLayouterOptions::ALGORITHM_ID,
        DisCoOptions::ALGORITHM_ID,
        FixedLayouterOptions::ALGORITHM_ID,
        ForceOptions::ALGORITHM_ID,
        LayeredOptions::ALGORITHM_ID,
        MrTreeOptions::ALGORITHM_ID,
        RadialOptions::ALGORITHM_ID,
        RandomLayouterOptions::ALGORITHM_ID,
        RectPackingOptions::ALGORITHM_ID,
        SporeCompactionOptions::ALGORITHM_ID,
        SporeOverlapRemovalOptions::ALGORITHM_ID,
        StressOptions::ALGORITHM_ID,
    ];

    for algorithm_id in algorithm_ids {
        assert!(
            service.get_algorithm_data(algorithm_id).is_some(),
            "algorithm not registered: {algorithm_id}"
        );
    }
}
