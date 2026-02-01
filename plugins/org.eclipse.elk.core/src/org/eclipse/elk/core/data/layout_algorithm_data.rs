#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LayoutAlgorithmData {
    id: String,
}

impl LayoutAlgorithmData {
    pub fn new(id: impl Into<String>) -> Self {
        LayoutAlgorithmData { id: id.into() }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}
