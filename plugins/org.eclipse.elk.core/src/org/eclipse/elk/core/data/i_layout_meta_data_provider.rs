use std::sync::Arc;
use std::any::Any;

use crate::org::eclipse::elk::core::data::{LayoutAlgorithmData, LayoutCategoryData, LayoutOptionData};

pub trait ILayoutMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry);
}

pub trait LayoutMetaDataRegistry {
    fn register_algorithm(&mut self, algorithm_data: LayoutAlgorithmData);
    fn register_option(&mut self, option_data: LayoutOptionData);
    fn register_category(&mut self, category_data: LayoutCategoryData);

    fn add_dependency(
        &mut self,
        source_option: &str,
        target_option: &str,
        required_value: Option<Arc<dyn Any + Send + Sync>>,
    );

    fn add_option_support(
        &mut self,
        algorithm: &str,
        option: &str,
        default_value: Option<Arc<dyn Any + Send + Sync>>,
    );
}
