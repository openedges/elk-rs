use crate::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;

use super::instance_pool::IFactory;

pub struct AlgorithmFactory {
    creator: Box<dyn Fn() -> Box<dyn AbstractLayoutProvider> + Send + Sync>,
    parameter: Option<String>,
}

impl AlgorithmFactory {
    pub fn new<F>(creator: F) -> Self
    where
        F: Fn() -> Box<dyn AbstractLayoutProvider> + Send + Sync + 'static,
    {
        AlgorithmFactory {
            creator: Box::new(creator),
            parameter: None,
        }
    }

    pub fn with_parameter<F>(creator: F, parameter: impl Into<String>) -> Self
    where
        F: Fn() -> Box<dyn AbstractLayoutProvider> + Send + Sync + 'static,
    {
        AlgorithmFactory {
            creator: Box::new(creator),
            parameter: Some(parameter.into()),
        }
    }
}

impl IFactory<Box<dyn AbstractLayoutProvider>> for AlgorithmFactory {
    fn create(&self) -> Box<dyn AbstractLayoutProvider> {
        let mut provider = (self.creator)();
        if let Some(parameter) = &self.parameter {
            provider.initialize(parameter);
        } else {
            provider.initialize("");
        }
        provider
    }

    fn destroy(&self, mut obj: Box<dyn AbstractLayoutProvider>) {
        obj.dispose();
    }
}
