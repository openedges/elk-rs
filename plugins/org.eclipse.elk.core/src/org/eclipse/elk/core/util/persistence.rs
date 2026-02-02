use crate::org::eclipse::elk::core::data::LayoutMetaDataService;
use crate::org::eclipse::elk::core::util::internal::LayoutOptionProxy;

#[derive(Clone, Debug)]
pub struct ElkGraphResource {
    encoding: String,
}

impl ElkGraphResource {
    pub fn new() -> Self {
        ElkGraphResource {
            encoding: "utf-8".to_string(),
        }
    }

    pub fn encoding(&self) -> &str {
        &self.encoding
    }

    pub fn set_encoding(&mut self, encoding: impl Into<String>) {
        self.encoding = encoding.into();
    }
}

impl Default for ElkGraphResource {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Default)]
pub struct ElkGraphResourceFactory;

impl ElkGraphResourceFactory {
    pub fn new() -> Self {
        ElkGraphResourceFactory
    }

    pub fn create_resource(&self) -> ElkGraphResource {
        ElkGraphResource::new()
    }
}

#[derive(Clone, Debug, Default)]
pub struct ElkGraphXMIHelper;

impl ElkGraphXMIHelper {
    pub fn new() -> Self {
        ElkGraphXMIHelper
    }

    pub fn create_property_id(&self, id: &str) -> String {
        id.to_string()
    }

    pub fn create_property_value(&self, value: &str) -> LayoutOptionProxy {
        LayoutOptionProxy::new(value)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ElkGraphXMISave;

impl ElkGraphXMISave {
    pub fn new() -> Self {
        ElkGraphXMISave
    }

    pub fn should_serialize_property(&self, property_id: &str) -> bool {
        LayoutMetaDataService::get_instance()
            .get_option_data(property_id)
            .map(|option_data| option_data.can_parse_value())
            .unwrap_or(false)
    }
}
