use std::any::Any;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;

use crate::org::eclipse::elk::core::data::LayoutMetaDataService;
use crate::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use crate::org::eclipse::elk::core::layout_configurator::LayoutConfigurator;
use crate::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use crate::org::eclipse::elk::core::service::{
    IDiagramLayoutConnector, LayoutConfigurationManager, LayoutConnectorsService, LayoutMapping,
};
use crate::org::eclipse::elk::core::util::{ElkUtil, IElkProgressMonitor, NullElkProgressMonitor};

pub struct DiagramLayoutEngine {
    config_manager: LayoutConfigurationManager,
}

impl Default for DiagramLayoutEngine {
    fn default() -> Self {
        DiagramLayoutEngine {
            config_manager: LayoutConfigurationManager::new(),
        }
    }
}

impl DiagramLayoutEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_configuration_provider(
        &mut self,
        provider: Option<
            Box<dyn crate::org::eclipse::elk::core::service::ILayoutConfigurationStoreProvider>,
        >,
    ) {
        self.config_manager.set_config_provider(provider);
    }

    pub fn invoke_layout(
        &mut self,
        connector: &dyn IDiagramLayoutConnector,
        workbench_part: Option<&dyn Any>,
        diagram_part: Option<&dyn Any>,
        params: Option<Parameters>,
    ) -> Option<LayoutMapping> {
        LayoutMetaDataService::get_instance();
        let mut params = params.unwrap_or_default();
        if params.configurators.is_empty() {
            params.add_layout_run();
        }

        let mut mapping = connector.build_layout_graph(workbench_part, diagram_part)?;
        mapping
            .properties_mut()
            .copy_properties(&params.global_settings);

        let Some(layout_graph) = mapping.layout_graph() else {
            return Some(mapping);
        };

        let mut progress_monitor = NullElkProgressMonitor;

        let mut config_configurator = self.maybe_create_configurator(&mapping);
        if params.override_diagram_config {
            if let Some(configurator) = config_configurator.as_mut() {
                apply_configurator(&layout_graph, configurator);
            }
            for configurator in params.configurators.iter_mut() {
                apply_configurator(&layout_graph, configurator);
            }
        } else {
            for configurator in params.configurators.iter_mut() {
                apply_configurator(&layout_graph, configurator);
            }
            if let Some(configurator) = config_configurator.as_mut() {
                apply_configurator(&layout_graph, configurator);
            }
        }

        let mut layout_engine = RecursiveGraphLayoutEngine::new();
        layout_engine.layout(&layout_graph, &mut progress_monitor);

        connector.apply_layout(&mut mapping, &params.global_settings);
        Some(mapping)
    }

    pub fn invoke_layout_with_service(
        &mut self,
        service: &LayoutConnectorsService,
        workbench_part: Option<&dyn Any>,
        diagram_part: Option<&dyn Any>,
        params: Option<Parameters>,
    ) -> Option<LayoutMapping> {
        LayoutMetaDataService::get_instance();
        let connector = service.get_connector(workbench_part, diagram_part)?;
        self.set_configuration_provider(
            service.get_configuration_provider(workbench_part, diagram_part),
        );

        let mut params = params.unwrap_or_default();
        if params.configurators.is_empty() {
            params.add_layout_run();
        }

        let mut mapping = connector.build_layout_graph(workbench_part, diagram_part)?;
        mapping
            .properties_mut()
            .copy_properties(&params.global_settings);

        let Some(layout_graph) = mapping.layout_graph() else {
            return Some(mapping);
        };

        let mut progress_monitor = NullElkProgressMonitor;
        service.fire_layout_about_to_start(&mapping, &mut progress_monitor);

        let mut config_configurator = self.maybe_create_configurator(&mapping);
        if params.override_diagram_config {
            if let Some(configurator) = config_configurator.as_mut() {
                apply_configurator(&layout_graph, configurator);
            }
            for configurator in params.configurators.iter_mut() {
                apply_configurator(&layout_graph, configurator);
            }
        } else {
            for configurator in params.configurators.iter_mut() {
                apply_configurator(&layout_graph, configurator);
            }
            if let Some(configurator) = config_configurator.as_mut() {
                apply_configurator(&layout_graph, configurator);
            }
        }

        let mut layout_engine = RecursiveGraphLayoutEngine::new();
        layout_engine.layout(&layout_graph, &mut progress_monitor);

        connector.apply_layout(&mut mapping, &params.global_settings);
        service.fire_layout_done(&mapping, &mut progress_monitor);
        Some(mapping)
    }

    fn maybe_create_configurator(&self, mapping: &LayoutMapping) -> Option<LayoutConfigurator> {
        if self.config_manager.has_provider() {
            Some(self.config_manager.create_configurator(mapping))
        } else {
            None
        }
    }
}

pub struct Parameters {
    configurators: Vec<LayoutConfigurator>,
    global_settings: MapPropertyHolder,
    override_diagram_config: bool,
}

impl Default for Parameters {
    fn default() -> Self {
        Parameters {
            configurators: Vec::new(),
            global_settings: MapPropertyHolder::new(),
            override_diagram_config: true,
        }
    }
}

impl Parameters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_override_diagram_config(&mut self, override_config: bool) -> &mut Self {
        self.override_diagram_config = override_config;
        self
    }

    pub fn global_settings(&mut self) -> &mut MapPropertyHolder {
        &mut self.global_settings
    }

    pub fn add_layout_run(&mut self) -> &mut LayoutConfigurator {
        let mut configurator = LayoutConfigurator::new();
        configurator.add_filter(
            crate::org::eclipse::elk::core::layout_configurator::OPTION_TARGET_FILTER.clone(),
        );
        self.configurators.push(configurator);
        self.configurators.last_mut().expect("just pushed")
    }
}

fn apply_configurator(
    layout_graph: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef,
    configurator: &mut LayoutConfigurator,
) {
    let mut visitors: Vec<&mut dyn crate::org::eclipse::elk::core::util::IGraphElementVisitor> =
        vec![configurator];
    ElkUtil::apply_visitors(layout_graph, &mut visitors);
}

impl DiagramLayoutEngine {
    pub fn run_with_progress(
        &mut self,
        connector: &dyn IDiagramLayoutConnector,
        workbench_part: Option<&dyn Any>,
        diagram_part: Option<&dyn Any>,
        params: Option<Parameters>,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) -> Option<LayoutMapping> {
        LayoutMetaDataService::get_instance();
        let mut params = params.unwrap_or_default();
        if params.configurators.is_empty() {
            params.add_layout_run();
        }

        let mut mapping = connector.build_layout_graph(workbench_part, diagram_part)?;
        mapping
            .properties_mut()
            .copy_properties(&params.global_settings);

        let Some(layout_graph) = mapping.layout_graph() else {
            return Some(mapping);
        };

        let mut config_configurator = self.maybe_create_configurator(&mapping);
        if params.override_diagram_config {
            if let Some(configurator) = config_configurator.as_mut() {
                apply_configurator(&layout_graph, configurator);
            }
            for configurator in params.configurators.iter_mut() {
                apply_configurator(&layout_graph, configurator);
            }
        } else {
            for configurator in params.configurators.iter_mut() {
                apply_configurator(&layout_graph, configurator);
            }
            if let Some(configurator) = config_configurator.as_mut() {
                apply_configurator(&layout_graph, configurator);
            }
        }

        let mut layout_engine = RecursiveGraphLayoutEngine::new();
        layout_engine.layout(&layout_graph, progress_monitor);

        connector.apply_layout(&mut mapping, &params.global_settings);
        Some(mapping)
    }
}
