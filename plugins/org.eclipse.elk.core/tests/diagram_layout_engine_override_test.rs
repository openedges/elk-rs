use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::service::{
    DiagramLayoutEngine, IDiagramLayoutConnector, ILayoutConfigurationStore,
    ILayoutConfigurationStoreProvider, LayoutMapping,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

#[derive(Clone)]
struct SimpleStore {
    value: String,
}

impl ILayoutConfigurationStore for SimpleStore {
    fn get_option_value(&self, option_id: &str) -> Option<String> {
        if option_id == CoreOptions::PRIORITY.id() {
            Some(self.value.clone())
        } else {
            None
        }
    }

    fn set_option_value(&mut self, _option_id: &str, _value: Option<String>) {}

    fn affected_options(&self) -> Vec<String> {
        vec![CoreOptions::PRIORITY.id().to_string()]
    }

    fn option_targets(&self) -> Vec<org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutOptionTarget> {
        vec![org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutOptionTarget::Parents]
    }

    fn parent(&self) -> Option<Box<dyn ILayoutConfigurationStore>> {
        None
    }

    fn clone_box(&self) -> Box<dyn ILayoutConfigurationStore> {
        Box::new(self.clone())
    }
}

struct SimpleProvider {
    store: SimpleStore,
}

impl ILayoutConfigurationStoreProvider for SimpleProvider {
    fn get(
        &self,
        _workbench_part: Option<&dyn Any>,
        _context: Option<&dyn Any>,
    ) -> Option<Box<dyn ILayoutConfigurationStore>> {
        Some(Box::new(self.store.clone()))
    }
}

struct DummyConnector;

impl IDiagramLayoutConnector for DummyConnector {
    fn build_layout_graph(
        &self,
        _workbench_part: Option<&dyn Any>,
        _diagram_part: Option<&dyn Any>,
    ) -> Option<LayoutMapping> {
        let root = ElkGraphUtil::create_graph();
        let mut mapping = LayoutMapping::new(None);
        mapping.set_layout_graph(root);
        Some(mapping)
    }

    fn apply_layout(&self, _mapping: &mut LayoutMapping, _settings: &MapPropertyHolder) {}
}

#[test]
fn override_order_applies_config_before_params() {
    let connector = DummyConnector;
    let mut engine = DiagramLayoutEngine::new();
    engine.set_configuration_provider(Some(Box::new(SimpleProvider {
        store: SimpleStore {
            value: "11".to_string(),
        },
    })));

    let mut params = org_eclipse_elk_core::org::eclipse::elk::core::service::DiagramLayoutParameters::new();
    params
        .add_layout_run()
        .configure_class(org_eclipse_elk_core::org::eclipse::elk::core::layout_configurator::LayoutConfiguratorClass::GraphElement)
        .set_property(CoreOptions::PRIORITY, Some(22_i32));

    let mapping = engine
        .invoke_layout(&connector, None, None, Some(params))
        .expect("mapping");
    let root = mapping.layout_graph().unwrap();
    let priority = root
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(CoreOptions::PRIORITY)
        .expect("priority");
    assert_eq!(priority, 22);
}

#[test]
fn override_false_applies_params_before_config() {
    let connector = DummyConnector;
    let mut engine = DiagramLayoutEngine::new();
    engine.set_configuration_provider(Some(Box::new(SimpleProvider {
        store: SimpleStore {
            value: "33".to_string(),
        },
    })));

    let mut params = org_eclipse_elk_core::org::eclipse::elk::core::service::DiagramLayoutParameters::new();
    params.set_override_diagram_config(false);
    params
        .add_layout_run()
        .configure_class(org_eclipse_elk_core::org::eclipse::elk::core::layout_configurator::LayoutConfiguratorClass::GraphElement)
        .set_property(CoreOptions::PRIORITY, Some(44_i32));

    let mapping = engine
        .invoke_layout(&connector, None, None, Some(params))
        .expect("mapping");
    let root = mapping.layout_graph().unwrap();
    let priority = root
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(CoreOptions::PRIORITY)
        .expect("priority");
    assert_eq!(priority, 33);
}
