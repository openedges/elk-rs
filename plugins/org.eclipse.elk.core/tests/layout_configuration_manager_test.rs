use std::collections::HashMap;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutOptionTarget;
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::service::{
    ILayoutConfigurationStore, ILayoutConfigurationStoreProvider, LayoutConfigurationManager,
    LayoutMapping,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

#[derive(Clone, Default)]
struct TestStore {
    values: HashMap<String, String>,
    targets: Vec<LayoutOptionTarget>,
    parent: Option<Box<TestStore>>,
}

impl ILayoutConfigurationStore for TestStore {
    fn get_option_value(&self, option_id: &str) -> Option<String> {
        self.values.get(option_id).cloned()
    }

    fn set_option_value(&mut self, option_id: &str, value: Option<String>) {
        if let Some(value) = value {
            self.values.insert(option_id.to_string(), value);
        } else {
            self.values.remove(option_id);
        }
    }

    fn affected_options(&self) -> Vec<String> {
        self.values.keys().cloned().collect()
    }

    fn option_targets(&self) -> Vec<LayoutOptionTarget> {
        self.targets.clone()
    }

    fn parent(&self) -> Option<Box<dyn ILayoutConfigurationStore>> {
        self.parent
            .as_ref()
            .map(|parent| Box::new((**parent).clone()) as Box<dyn ILayoutConfigurationStore>)
    }
}

struct TestProvider {
    store: TestStore,
}

impl ILayoutConfigurationStoreProvider for TestProvider {
    fn get(
        &self,
        _workbench_part: Option<&dyn std::any::Any>,
        _context: Option<&dyn std::any::Any>,
    ) -> Option<Box<dyn ILayoutConfigurationStore>> {
        Some(Box::new(self.store.clone()))
    }
}

#[test]
fn layout_configuration_manager_resolves_values() {
    let mut store = TestStore {
        targets: vec![LayoutOptionTarget::Parents],
        ..Default::default()
    };
    store
        .values
        .insert(CoreOptions::ALGORITHM.id().to_string(), "layered".to_string());
    store.values.insert(
        CoreOptions::SPACING_NODE_NODE.id().to_string(),
        "42".to_string(),
    );

    let manager = LayoutConfigurationManager::new();
    let algo = manager.get_algorithm(&store).expect("algorithm");
    assert_eq!(algo.id(), "org.eclipse.elk.layered");

    let option_data = org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService::get_instance()
        .get_option_data(CoreOptions::SPACING_NODE_NODE.id())
        .expect("option data");
    let value = manager
        .get_option_value(&option_data, &store)
        .expect("value");
    let spacing = value.downcast_ref::<f64>().expect("f64");
    assert!((*spacing - 42.0).abs() < f64::EPSILON);
}

#[test]
fn layout_configuration_manager_applies_configurator() {
    let mut store = TestStore {
        targets: vec![LayoutOptionTarget::Parents],
        ..Default::default()
    };
    store.values.insert(
        CoreOptions::SPACING_NODE_NODE.id().to_string(),
        "25".to_string(),
    );

    let provider = TestProvider { store };
    let mut manager = LayoutConfigurationManager::new();
    manager.set_config_provider(Some(Box::new(provider)));

    let root = ElkGraphUtil::create_graph();
    let mut mapping = LayoutMapping::new(None);
    mapping.set_layout_graph(root.clone());

    let mut configurator = manager.create_configurator(&mapping);
    let mut visitors: Vec<&mut dyn org_eclipse_elk_core::org::eclipse::elk::core::util::IGraphElementVisitor> =
        vec![&mut configurator];
    ElkUtil::apply_visitors(&root, &mut visitors);

    let value = root
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(CoreOptions::SPACING_NODE_NODE)
        .expect("spacing");
    assert!((value - 25.0).abs() < f64::EPSILON);
}
