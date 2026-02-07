use std::sync::OnceLock;

use crate::org::eclipse::elk::alg::layered::options::LayeredMetaDataProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;

/// Mirrors Java `PlainJavaInitialization.initializePlainJavaLayout()` for layered tests.
pub fn initialize_plain_java_layout() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
    });
}
