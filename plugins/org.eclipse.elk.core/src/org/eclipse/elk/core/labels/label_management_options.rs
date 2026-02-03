use std::any::TypeId;
use std::collections::HashSet;
use std::sync::Arc;

use crate::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutMetaDataRegistry, LayoutOptionData, LayoutOptionTarget,
    LayoutOptionType, LayoutOptionVisibility,
};
use crate::org::eclipse::elk::core::labels::ILabelManager;
use crate::org::eclipse::elk::core::options::CoreOptions;

pub struct LabelManagementOptions;

impl ILayoutMetaDataProvider for LabelManagementOptions {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        let targets = HashSet::from([
            LayoutOptionTarget::Parents,
            LayoutOptionTarget::Labels,
        ]);
        let option = LayoutOptionData::builder()
            .id(CoreOptions::LABEL_MANAGER.id())
            .option_type(LayoutOptionType::Object)
            .name("Label Manager")
            .description(
                "Label managers can shorten labels upon a layout algorithm's request.",
            )
            .targets(targets)
            .visibility(LayoutOptionVisibility::Hidden)
            .value_type_id(TypeId::of::<Arc<dyn ILabelManager>>())
            .create();
        registry.register_option(option);
    }
}
