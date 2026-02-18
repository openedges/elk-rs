use crate::org::eclipse::elk::core::comments::i_data_provider::IDataProvider;

pub trait IExplicitAttachmentProvider<C, T> {
    fn find_explicit_attachment(&self, comment: &C) -> Option<T>;

    fn preprocess(&self, _data_provider: &dyn IDataProvider<C, T>, _include_hierarchy: bool) {}

    fn cleanup(&self) {}
}
