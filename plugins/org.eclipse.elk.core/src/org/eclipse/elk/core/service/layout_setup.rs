use std::any::Any;

use crate::org::eclipse::elk::core::service::{
    IDiagramLayoutConnector, ILayoutConfigurationStoreProvider,
};

pub trait ILayoutSetup {
    fn supports(&self, object: &dyn Any) -> bool;
    fn create_connector(&self) -> Box<dyn IDiagramLayoutConnector>;
    fn configuration_store_provider(&self) -> Option<Box<dyn ILayoutConfigurationStoreProvider>> {
        None
    }
}
