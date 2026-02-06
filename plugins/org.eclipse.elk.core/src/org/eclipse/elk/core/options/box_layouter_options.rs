use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutMetaDataRegistry, LayoutOptionData, LayoutOptionTarget,
    LayoutOptionType,
};
use crate::org::eclipse::elk::core::math::{ElkPadding, KVector};
use crate::org::eclipse::elk::core::options::{CoreOptions, PackingMode, SizeConstraint, SizeOptions};
use crate::org::eclipse::elk::core::util::EnumSet;

pub struct BoxLayouterOptions;

impl BoxLayouterOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.box";

    pub const PADDING: &'static LazyLock<Property<ElkPadding>> = CoreOptions::PADDING;
    pub const SPACING_NODE_NODE: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_NODE_NODE;
    pub const PRIORITY: &'static LazyLock<Property<i32>> = CoreOptions::PRIORITY;
    pub const EXPAND_NODES: &'static LazyLock<Property<bool>> = CoreOptions::EXPAND_NODES;
    pub const INTERACTIVE: &'static LazyLock<Property<bool>> = CoreOptions::INTERACTIVE;
    pub const NODE_SIZE_CONSTRAINTS: &'static LazyLock<Property<EnumSet<SizeConstraint>>> =
        CoreOptions::NODE_SIZE_CONSTRAINTS;
    pub const NODE_SIZE_OPTIONS: &'static LazyLock<Property<EnumSet<SizeOptions>>> =
        CoreOptions::NODE_SIZE_OPTIONS;
    pub const ASPECT_RATIO: &'static LazyLock<Property<f64>> = CoreOptions::ASPECT_RATIO;
    pub const NODE_SIZE_MINIMUM: &'static LazyLock<Property<KVector>> = CoreOptions::NODE_SIZE_MINIMUM;
    pub const BOX_PACKING_MODE: &'static LazyLock<Property<PackingMode>> =
        CoreOptions::BOX_PACKING_MODE;
}

impl ILayoutMetaDataProvider for BoxLayouterOptions {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        let targets = [LayoutOptionTarget::Parents];
        let option_data = LayoutOptionData::builder()
            .id(Self::BOX_PACKING_MODE.id())
            .option_type(LayoutOptionType::Enum)
            .targets(targets.iter().copied().collect())
            .default_value(Some(std::sync::Arc::new(PackingMode::Simple)))
            .choices(vec![
                "SIMPLE".to_string(),
                "GROUP_DEC".to_string(),
                "GROUP_MIXED".to_string(),
                "GROUP_INC".to_string(),
            ])
            .value_type_id(std::any::TypeId::of::<PackingMode>())
            .parser(std::sync::Arc::new(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return None;
                }
                let normalized = trimmed
                    .chars()
                    .filter(|c| *c != '_')
                    .collect::<String>()
                    .to_ascii_uppercase();
                let parsed = match normalized.as_str() {
                    "SIMPLE" => PackingMode::Simple,
                    "GROUPDEC" => PackingMode::GroupDec,
                    "GROUPMIXED" => PackingMode::GroupMixed,
                    "GROUPINC" => PackingMode::GroupInc,
                    _ => {
                        if let Ok(index) = trimmed.parse::<usize>() {
                            match index {
                                0 => PackingMode::Simple,
                                1 => PackingMode::GroupDec,
                                2 => PackingMode::GroupMixed,
                                3 => PackingMode::GroupInc,
                                _ => return None,
                            }
                        } else {
                            return None;
                        }
                    }
                };
                Some(std::sync::Arc::new(parsed) as std::sync::Arc<dyn std::any::Any + Send + Sync>)
            }))
            .create();
        registry.register_option(option_data);
    }
}
