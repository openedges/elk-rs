use std::any::Any;
use std::fmt;
use std::sync::Arc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{
    IPropertyValueProxy, MapPropertyHolder,
};

use crate::org::eclipse::elk::core::data::LayoutMetaDataService;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LayoutOptionProxy {
    value: String,
}

impl LayoutOptionProxy {
    pub fn new(value: impl Into<String>) -> Self {
        LayoutOptionProxy {
            value: value.into(),
        }
    }

    pub fn set_proxy_value(property_holder: &mut MapPropertyHolder, key: &str, value: &str) {
        let proxy = LayoutOptionProxy::new(value);
        property_holder.set_property_proxy(key.to_string(), Arc::new(proxy));
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

impl IPropertyValueProxy for LayoutOptionProxy {
    fn resolve_value(&self, property_id: &str) -> Option<Arc<dyn Any + Send + Sync>> {
        LayoutMetaDataService::get_instance()
            .get_option_data(property_id)
            .and_then(|option_data| option_data.parse_value(&self.value))
    }
}

impl fmt::Display for LayoutOptionProxy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
