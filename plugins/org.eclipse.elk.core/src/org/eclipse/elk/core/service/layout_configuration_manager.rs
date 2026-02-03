use std::any::Any;
use std::collections::HashSet;
use std::sync::Arc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::GraphFeature;

use crate::org::eclipse::elk::core::data::{
    LayoutAlgorithmData, LayoutAlgorithmResolver, LayoutMetaDataService, LayoutOptionData,
    LayoutOptionTarget,
};
use crate::org::eclipse::elk::core::layout_configurator::LayoutConfigurator;
use crate::org::eclipse::elk::core::options::{CoreOptions, HierarchyHandling};
use crate::org::eclipse::elk::core::service::{
    ILayoutConfigurationStore, ILayoutConfigurationStoreProvider, LayoutMapping,
};
use crate::org::eclipse::elk::core::util::{ElkUtil, IGraphElementVisitor};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef;

#[derive(Default)]
pub struct LayoutConfigurationManager {
    config_provider: Option<Box<dyn ILayoutConfigurationStoreProvider>>,
    layout_algorithm_resolver: LayoutAlgorithmResolver,
}

impl LayoutConfigurationManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_config_provider(
        &mut self,
        provider: Option<Box<dyn ILayoutConfigurationStoreProvider>>,
    ) {
        self.config_provider = provider;
    }

    pub fn has_provider(&self) -> bool {
        self.config_provider.is_some()
    }

    pub fn get_algorithm(&self, config: &dyn ILayoutConfigurationStore) -> Option<LayoutAlgorithmData> {
        let algorithm_id = config.get_option_value(CoreOptions::ALGORITHM.id());
        LayoutMetaDataService::get_instance().get_algorithm_data_by_suffix_or_default(
            algorithm_id.as_deref(),
            Some(self.layout_algorithm_resolver.default_layout_algorithm_id()),
        )
    }

    pub fn get_supported_options(
        &self,
        config: &dyn ILayoutConfigurationStore,
    ) -> HashSet<LayoutOptionData> {
        let layout_data_service = LayoutMetaDataService::get_instance();
        let mut option_data = HashSet::new();
        let targets = config.option_targets();

        if targets.contains(&LayoutOptionTarget::Parents) {
            if let Some(algo_data) = self.get_algorithm(config) {
                option_data.extend(layout_data_service.get_option_data_for_algorithm(
                    &algo_data,
                    LayoutOptionTarget::Parents,
                ));
            }
        }

        if let Some(parent_config) = config.parent() {
            if let Some(algo_data) = self.get_algorithm(parent_config.as_ref()) {
                for target in targets.iter().copied() {
                    if target != LayoutOptionTarget::Parents {
                        option_data.extend(layout_data_service.get_option_data_for_algorithm(
                            &algo_data,
                            target,
                        ));
                    }
                }
            }
        }

        option_data
    }

    pub fn get_option_value(
        &self,
        option_data: &LayoutOptionData,
        config: &dyn ILayoutConfigurationStore,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        let mut result = self.get_raw_option_value(option_data, config);

        if option_data.id() == CoreOptions::ALGORITHM.id() {
            let algorithm_id = result
                .as_ref()
                .and_then(|value| value.downcast_ref::<String>())
                .map(|value| value.as_str());
            if let Some(algo_data) = LayoutMetaDataService::get_instance()
                .get_algorithm_data_by_suffix_or_default(
                    algorithm_id,
                    Some(self.layout_algorithm_resolver.default_layout_algorithm_id()),
                )
            {
                return Some(Arc::new(algo_data.id().to_string()));
            }
        } else if result.is_some() {
            return result;
        }

        if option_data.targets().contains(&LayoutOptionTarget::Parents) {
            if let Some(algo_data) = self.get_algorithm(config) {
                result = algo_data.default_value_any(option_data.id());
                if result.is_some() {
                    return result;
                }
            }
        } else if let Some(parent_config) = config.parent() {
            if let Some(algo_data) = self.get_algorithm(parent_config.as_ref()) {
                result = algo_data.default_value_any(option_data.id());
                if result.is_some() {
                    return result;
                }
            }
        }

        result = option_data.default_value();
        if result.is_some() {
            return result;
        }

        option_data.default_default_value()
    }

    pub fn clear_option_values(&self, config: &mut dyn ILayoutConfigurationStore) {
        for option_id in config.affected_options() {
            config.set_option_value(&option_id, None);
        }
    }

    pub fn create_configurator(&self, layout_mapping: &LayoutMapping) -> LayoutConfigurator {
        let mut result = LayoutConfigurator::new();
        let Some(provider) = &self.config_provider else {
            return result;
        };

        let Some(root) = layout_mapping.layout_graph() else {
            return result;
        };

        let mut visitor = ConfigVisitor {
            provider: provider.as_ref(),
            mapping: layout_mapping,
            configurator: &mut result,
        };
        let mut visitors: Vec<&mut dyn IGraphElementVisitor> = vec![&mut visitor];
        ElkUtil::apply_visitors(&root, &mut visitors);
        result
    }

    fn get_raw_option_value(
        &self,
        option_data: &LayoutOptionData,
        config: &dyn ILayoutConfigurationStore,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        let raw = config.get_option_value(option_data.id())?;
        option_data.parse_value(&raw)
    }

    pub fn get_responsible_parent(
        &self,
        config: &dyn ILayoutConfigurationStore,
    ) -> Option<Box<dyn ILayoutConfigurationStore>> {
        let mut chain = Vec::new();
        let mut current = config.parent();
        while let Some(store) = current {
            current = store.parent();
            chain.push(store);
        }
        if chain.is_empty() {
            return None;
        }

        let mut chosen_index = 0;
        for (index, store) in chain.iter().enumerate() {
            if self.is_full_hierarchy_layout(store.as_ref()) {
                chosen_index = index;
            }
        }
        Some(chain.swap_remove(chosen_index))
    }

    pub fn get_root(
        &self,
        config: &dyn ILayoutConfigurationStore,
    ) -> Box<dyn ILayoutConfigurationStore> {
        let mut current = config.clone_box();
        loop {
            match current.parent() {
                Some(parent) => {
                    current = parent;
                }
                None => return current,
            }
        }
    }

    fn is_full_hierarchy_layout(&self, config: &dyn ILayoutConfigurationStore) -> bool {
        let raw = config.get_option_value(CoreOptions::HIERARCHY_HANDLING.id());
        let Some(raw) = raw else {
            return false;
        };
        let option_data = LayoutMetaDataService::get_instance()
            .get_option_data(CoreOptions::HIERARCHY_HANDLING.id());
        let Some(option_data) = option_data else {
            return false;
        };
        let parsed = option_data.parse_value(&raw);
        let Some(parsed) = parsed else {
            return false;
        };
        let hierarchy = parsed.downcast_ref::<HierarchyHandling>();
        if hierarchy != Some(&HierarchyHandling::IncludeChildren) {
            return false;
        }

        let algo_data = self.get_algorithm(config);
        let Some(algo_data) = algo_data else {
            return false;
        };
        algo_data.supports_feature(GraphFeature::Compound)
            || algo_data.supports_feature(GraphFeature::Clusters)
    }
}

struct ConfigVisitor<'a> {
    provider: &'a dyn ILayoutConfigurationStoreProvider,
    mapping: &'a LayoutMapping,
    configurator: &'a mut LayoutConfigurator,
}

impl<'a> IGraphElementVisitor for ConfigVisitor<'a> {
    fn visit(&mut self, element: &ElkGraphElementRef) {
        let diagram = self.mapping.diagram_for(element);
        let workbench = self.mapping.workbench_part();
        let store = self.provider.get(
            workbench.as_ref().map(|value| value.as_ref()),
            diagram.as_ref().map(|value| value.as_ref()),
        );
        let Some(store) = store else {
            return;
        };
        configure_element(element, store.as_ref(), self.configurator);
    }
}

fn configure_element(
    element: &ElkGraphElementRef,
    config_store: &dyn ILayoutConfigurationStore,
    configurator: &mut LayoutConfigurator,
) {
    let layout_data_service = LayoutMetaDataService::get_instance();
    for option_id in config_store.affected_options() {
        let Some(value) = config_store.get_option_value(&option_id) else {
            continue;
        };
        let option_data = layout_data_service.get_option_data(&option_id);
        let Some(option_data) = option_data else {
            continue;
        };
        let parsed = option_data.parse_value(&value);
        let Some(parsed) = parsed else {
            continue;
        };
        configurator
            .configure_element(element)
            .set_property_any(option_data.id(), Some(parsed));
    }
}
