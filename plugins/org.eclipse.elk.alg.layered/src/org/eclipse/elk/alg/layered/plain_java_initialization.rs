use std::sync::OnceLock;

use crate::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use crate::org::eclipse::elk::alg::layered::options::LayeredMetaDataProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{AlgorithmFactory, InstancePool};
use std::sync::Arc;

/// Mirrors Java `PlainJavaInitialization.initializePlainJavaLayout()` for layered tests.
pub fn initialize_plain_java_layout() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
        let layered_factory = AlgorithmFactory::new(|| Box::new(LayeredLayoutProvider::new()));
        let layered_pool = InstancePool::new(Box::new(layered_factory));
        service.override_algorithm_provider_pool("org.eclipse.elk.layered", Arc::new(layered_pool));
    });
}
