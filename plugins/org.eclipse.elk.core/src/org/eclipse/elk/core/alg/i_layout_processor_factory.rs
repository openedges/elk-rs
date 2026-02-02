use std::any::{Any, TypeId};

use super::i_layout_processor::ILayoutProcessor;

pub trait ILayoutProcessorFactory<G>: Send + Sync {
    fn create(&self) -> Box<dyn ILayoutProcessor<G>>;

    fn as_any(&self) -> &dyn Any;

    fn enum_ordinal(&self) -> Option<usize> {
        None
    }

    fn enum_type_id(&self) -> Option<TypeId> {
        Some(self.as_any().type_id())
    }
}
