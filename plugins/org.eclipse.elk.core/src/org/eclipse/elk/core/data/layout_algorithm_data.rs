use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::GraphFeature;

use crate::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use crate::org::eclipse::elk::core::data::ILayoutMetaData;
use crate::org::eclipse::elk::core::util::InstancePool;
use crate::org::eclipse::elk::core::validation::IValidatingGraphElementVisitor;

pub type ValidatorFactory =
    Arc<dyn Fn() -> Box<dyn IValidatingGraphElementVisitor> + Send + Sync>;

#[derive(Clone)]
pub struct LayoutAlgorithmData {
    id: String,
    name: String,
    description: String,
    category_id: Option<String>,
    bundle_name: Option<String>,
    defining_bundle_id: Option<String>,
    preview_image_path: Option<String>,
    validator_factory: Option<ValidatorFactory>,
    provider_pool: Option<Arc<InstancePool<Box<dyn AbstractLayoutProvider>>>>,
    supported_features: HashSet<GraphFeature>,
    known_options: HashMap<String, Option<Arc<dyn Any + Send + Sync>>>,
}

impl LayoutAlgorithmData {
    pub fn new(id: impl Into<String>) -> Self {
        LayoutAlgorithmData {
            id: id.into(),
            name: String::new(),
            description: String::new(),
            category_id: None,
            bundle_name: None,
            defining_bundle_id: None,
            preview_image_path: None,
            validator_factory: None,
            provider_pool: None,
            supported_features: HashSet::new(),
            known_options: HashMap::new(),
        }
    }

    pub fn with_validator(id: impl Into<String>, factory: ValidatorFactory) -> Self {
        LayoutAlgorithmData {
            id: id.into(),
            name: String::new(),
            description: String::new(),
            category_id: None,
            bundle_name: None,
            defining_bundle_id: None,
            preview_image_path: None,
            validator_factory: Some(factory),
            provider_pool: None,
            supported_features: HashSet::new(),
            known_options: HashMap::new(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn category_id(&self) -> Option<&str> {
        self.category_id.as_deref()
    }

    pub fn bundle_name(&self) -> Option<&str> {
        self.bundle_name.as_deref()
    }

    pub fn defining_bundle_id(&self) -> Option<&str> {
        self.defining_bundle_id.as_deref()
    }

    pub fn preview_image_path(&self) -> Option<&str> {
        self.preview_image_path.as_deref()
    }

    pub fn set_name(&mut self, name: impl Into<String>) -> &mut Self {
        self.name = name.into();
        self
    }

    pub fn set_description(&mut self, description: impl Into<String>) -> &mut Self {
        self.description = description.into();
        self
    }

    pub fn set_category_id(&mut self, category_id: Option<impl Into<String>>) -> &mut Self {
        self.category_id = category_id.map(Into::into);
        self
    }

    pub fn set_bundle_name(&mut self, bundle_name: Option<impl Into<String>>) -> &mut Self {
        self.bundle_name = bundle_name.map(Into::into);
        self
    }

    pub fn set_defining_bundle_id(
        &mut self,
        defining_bundle_id: Option<impl Into<String>>,
    ) -> &mut Self {
        self.defining_bundle_id = defining_bundle_id.map(Into::into);
        self
    }

    pub fn set_preview_image_path(
        &mut self,
        preview_image_path: Option<impl Into<String>>,
    ) -> &mut Self {
        self.preview_image_path = preview_image_path.map(Into::into);
        self
    }

    pub fn validator_factory(&self) -> Option<&ValidatorFactory> {
        self.validator_factory.as_ref()
    }

    pub fn set_validator_factory(&mut self, factory: Option<ValidatorFactory>) -> &mut Self {
        self.validator_factory = factory;
        self
    }

    pub fn provider_pool(&self) -> Option<Arc<InstancePool<Box<dyn AbstractLayoutProvider>>>> {
        self.provider_pool.clone()
    }

    pub fn set_provider_pool(
        &mut self,
        pool: Option<Arc<InstancePool<Box<dyn AbstractLayoutProvider>>>>,
    ) -> &mut Self {
        self.provider_pool = pool;
        self
    }

    pub fn with_provider_pool(
        mut self,
        pool: Arc<InstancePool<Box<dyn AbstractLayoutProvider>>>,
    ) -> Self {
        self.provider_pool = Some(pool);
        self
    }

    pub fn add_known_option_id(&mut self, option_id: impl Into<String>) {
        self.known_options.insert(option_id.into(), None);
    }

    pub fn add_known_option_default(
        &mut self,
        option_id: impl Into<String>,
        default_value: Option<Arc<dyn Any + Send + Sync>>,
    ) {
        self.known_options.insert(option_id.into(), default_value);
    }

    pub fn knows_option(&self, option_id: &str) -> bool {
        self.known_options.contains_key(option_id)
    }

    pub fn known_option_ids(&self) -> impl Iterator<Item = &String> {
        self.known_options.keys()
    }

    pub fn default_value_any(&self, option_id: &str) -> Option<Arc<dyn Any + Send + Sync>> {
        self.known_options
            .get(option_id)
            .and_then(|value| value.as_ref().map(Arc::clone))
    }

    pub fn supports_feature(&self, feature: GraphFeature) -> bool {
        self.supported_features.contains(&feature)
    }

    pub fn supported_features(&self) -> &HashSet<GraphFeature> {
        &self.supported_features
    }

    pub fn add_supported_feature(&mut self, feature: GraphFeature) -> &mut Self {
        self.supported_features.insert(feature);
        self
    }

    pub fn set_supported_features(&mut self, features: HashSet<GraphFeature>) -> &mut Self {
        self.supported_features = features;
        self
    }
}

impl ILayoutMetaData for LayoutAlgorithmData {
    fn id(&self) -> &str {
        self.id()
    }

    fn name(&self) -> &str {
        self.name()
    }

    fn description(&self) -> &str {
        self.description()
    }
}

impl std::fmt::Debug for LayoutAlgorithmData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutAlgorithmData")
            .field("id", &self.id)
            .finish()
    }
}

impl PartialEq for LayoutAlgorithmData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for LayoutAlgorithmData {}

impl Hash for LayoutAlgorithmData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
