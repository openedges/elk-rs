pub trait ILayoutMetaData {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}
