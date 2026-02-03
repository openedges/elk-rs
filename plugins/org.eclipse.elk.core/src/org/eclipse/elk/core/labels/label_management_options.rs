use std::any::TypeId;
use std::collections::HashSet;
use std::sync::{Arc, LazyLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutMetaDataRegistry, LayoutOptionData, LayoutOptionTarget,
    LayoutOptionType, LayoutOptionVisibility,
};
use crate::org::eclipse::elk::core::labels::ILabelManager;

pub static LABEL_MANAGER_PROPERTY: LazyLock<Property<Arc<dyn ILabelManager>>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.labels.labelManager"));

pub struct LabelManagementOptions;

impl LabelManagementOptions {
    pub const LABEL_MANAGER: &'static LazyLock<Property<Arc<dyn ILabelManager>>> =
        &LABEL_MANAGER_PROPERTY;
}

impl ILayoutMetaDataProvider for LabelManagementOptions {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        let targets = HashSet::from([
            LayoutOptionTarget::Parents,
            LayoutOptionTarget::Labels,
        ]);
        let option = LayoutOptionData::builder()
            .id(Self::LABEL_MANAGER.id())
            .option_type(LayoutOptionType::Object)
            .name("Label Manager")
            .description(
                concat!(
                    "The label manager responsible for a given part of the graph. A label manager can either be ",
                    "attached to a compound node (in which case it is responsible for all labels inside) or to specific ",
                    "labels. The label manager can then be called by layout algorithms to modify labels that are too ",
                    "wide to try and shorten them to a given target width."
                ),
            )
            .targets(targets)
            .visibility(LayoutOptionVisibility::Hidden)
            .value_type_id(TypeId::of::<Arc<dyn ILabelManager>>())
            .create();
        registry.register_option(option);
    }
}
