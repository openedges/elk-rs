use std::any::Any;

use crate::org::eclipse::elk::core::data::LayoutOptionTarget;

pub trait ILayoutConfigurationStore {
    fn get_option_value(&self, option_id: &str) -> Option<String>;
    fn set_option_value(&mut self, option_id: &str, value: Option<String>);
    fn affected_options(&self) -> Vec<String>;
    fn option_targets(&self) -> Vec<LayoutOptionTarget>;
    fn parent(&self) -> Option<Box<dyn ILayoutConfigurationStore>>;
    fn clone_box(&self) -> Box<dyn ILayoutConfigurationStore>;
}

pub trait ILayoutConfigurationStoreProvider {
    fn get(
        &self,
        workbench_part: Option<&dyn Any>,
        context: Option<&dyn Any>,
    ) -> Option<Box<dyn ILayoutConfigurationStore>>;
}
