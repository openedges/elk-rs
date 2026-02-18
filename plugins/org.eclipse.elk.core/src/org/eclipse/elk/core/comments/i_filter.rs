use crate::org::eclipse::elk::core::comments::i_data_provider::IDataProvider;

pub trait IFilter<C, T> {
    fn eligible_for_attachment(&self, comment: &C) -> bool;

    fn preprocess(&self, _data_provider: &dyn IDataProvider<C, T>, _include_hierarchy: bool) {}

    fn cleanup(&self) {}
}
