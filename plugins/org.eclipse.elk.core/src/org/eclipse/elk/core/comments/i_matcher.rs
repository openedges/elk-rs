use std::any::TypeId;

use crate::org::eclipse::elk::core::comments::i_data_provider::IDataProvider;

pub trait IMatcher<C, T>: 'static {
    fn raw(&self, comment: &C, target: &T) -> f64;
    fn normalized(&self, comment: &C, target: &T) -> f64;

    fn preprocess(&self, _data_provider: &dyn IDataProvider<C, T>, _include_hierarchy: bool) {}

    fn cleanup(&self) {}

    fn matcher_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}
