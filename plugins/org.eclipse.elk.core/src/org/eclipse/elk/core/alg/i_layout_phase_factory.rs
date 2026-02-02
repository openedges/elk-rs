use std::any::{Any, TypeId};

use crate::org::eclipse::elk::core::util::EnumSetType;

use super::i_layout_phase::ILayoutPhase;

pub trait ILayoutPhaseFactory<P: EnumSetType, G>: Send + Sync {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<P, G>>;

    fn as_any(&self) -> &dyn Any;

    fn enum_ordinal(&self) -> Option<usize> {
        None
    }

    fn enum_type_id(&self) -> Option<TypeId> {
        Some(self.as_any().type_id())
    }
}
