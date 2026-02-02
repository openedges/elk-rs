use std::any::Any;
use std::sync::Arc;

use crate::org::eclipse::elk::core::service::{
    IDiagramLayoutConnector, ILayoutConfigurationStoreProvider, ILayoutListener, ILayoutSetup,
    LayoutMapping,
};
use crate::org::eclipse::elk::core::util::IElkProgressMonitor;

struct SetupEntry {
    priority: i32,
    setup: Box<dyn ILayoutSetup>,
    connector_factory: Arc<dyn Fn() -> Box<dyn IDiagramLayoutConnector> + Send + Sync>,
    config_provider_factory:
        Option<Arc<dyn Fn() -> Box<dyn ILayoutConfigurationStoreProvider> + Send + Sync>>,
}

pub struct LayoutConnectorsService {
    entries: Vec<SetupEntry>,
    listeners: Vec<Arc<dyn ILayoutListener>>,
}

impl Default for LayoutConnectorsService {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutConnectorsService {
    pub fn new() -> Self {
        LayoutConnectorsService {
            entries: Vec::new(),
            listeners: Vec::new(),
        }
    }

    pub fn register_setup(
        &mut self,
        priority: i32,
        setup: Box<dyn ILayoutSetup>,
        connector_factory: Arc<dyn Fn() -> Box<dyn IDiagramLayoutConnector> + Send + Sync>,
        config_provider_factory: Option<
            Arc<dyn Fn() -> Box<dyn ILayoutConfigurationStoreProvider> + Send + Sync>,
        >,
    ) {
        let entry = SetupEntry {
            priority,
            setup,
            connector_factory,
            config_provider_factory,
        };
        self.insert_sorted(entry);
    }

    pub fn get_connector(
        &self,
        workbench_part: Option<&dyn Any>,
        diagram_part: Option<&dyn Any>,
    ) -> Option<Box<dyn IDiagramLayoutConnector>> {
        for entry in &self.entries {
            if matches_support(entry, workbench_part, diagram_part) {
                return Some((entry.connector_factory)());
            }
        }
        None
    }

    pub fn get_configuration_provider(
        &self,
        workbench_part: Option<&dyn Any>,
        diagram_part: Option<&dyn Any>,
    ) -> Option<Box<dyn ILayoutConfigurationStoreProvider>> {
        for entry in &self.entries {
            if matches_support(entry, workbench_part, diagram_part) {
                if let Some(factory) = &entry.config_provider_factory {
                    return Some((factory)());
                }
                return None;
            }
        }
        None
    }

    pub fn add_layout_listener(&mut self, listener: Arc<dyn ILayoutListener>) {
        self.listeners.push(listener);
    }

    pub fn remove_layout_listener(&mut self, listener: &Arc<dyn ILayoutListener>) {
        self.listeners
            .retain(|existing| !Arc::ptr_eq(existing, listener));
    }

    pub fn fire_layout_about_to_start(
        &self,
        mapping: &LayoutMapping,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        for listener in &self.listeners {
            listener.layout_about_to_start(mapping, progress_monitor);
        }
    }

    pub fn fire_layout_done(
        &self,
        mapping: &LayoutMapping,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        for listener in &self.listeners {
            listener.layout_done(mapping, progress_monitor);
        }
    }

    fn insert_sorted(&mut self, entry: SetupEntry) {
        let index = self
            .entries
            .iter()
            .position(|existing| existing.priority <= entry.priority)
            .unwrap_or(self.entries.len());
        self.entries.insert(index, entry);
    }
}

fn matches_support(
    entry: &SetupEntry,
    workbench_part: Option<&dyn Any>,
    diagram_part: Option<&dyn Any>,
) -> bool {
    if let Some(workbench) = workbench_part {
        if !entry.setup.supports(workbench) {
            return false;
        }
        if let Some(diagram) = diagram_part {
            return entry.setup.supports(diagram);
        }
        return true;
    }

    if let Some(diagram) = diagram_part {
        return entry.setup.supports(diagram);
    }

    false
}
