use std::hash::{Hash, Hasher};

use crate::org::eclipse::elk::core::data::{ILayoutMetaData, LayoutAlgorithmData};

#[derive(Clone)]
pub struct LayoutCategoryData {
    id: String,
    name: String,
    description: String,
    layouters: Vec<LayoutAlgorithmData>,
}

impl LayoutCategoryData {
    pub fn builder() -> LayoutCategoryDataBuilder {
        LayoutCategoryDataBuilder::new()
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn layouters(&self) -> &[LayoutAlgorithmData] {
        &self.layouters
    }

    pub fn layouters_mut(&mut self) -> &mut Vec<LayoutAlgorithmData> {
        &mut self.layouters
    }
}

impl ILayoutMetaData for LayoutCategoryData {
    fn id(&self) -> &str {
        self.id()
    }

    fn name(&self) -> &str {
        self.name()
    }

    fn description(&self) -> &str {
        self.description()
    }
}

impl std::fmt::Debug for LayoutCategoryData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutCategoryData")
            .field("id", &self.id)
            .finish()
    }
}

impl PartialEq for LayoutCategoryData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for LayoutCategoryData {}

impl Hash for LayoutCategoryData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct LayoutCategoryDataBuilder {
    id: Option<String>,
    name: Option<String>,
    description: Option<String>,
}

impl LayoutCategoryDataBuilder {
    pub fn new() -> Self {
        LayoutCategoryDataBuilder {
            id: None,
            name: None,
            description: None,
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn create(self) -> LayoutCategoryData {
        LayoutCategoryData {
            id: self.id.unwrap_or_default(),
            name: self.name.unwrap_or_default(),
            description: self.description.unwrap_or_default(),
            layouters: Vec::new(),
        }
    }
}

impl Default for LayoutCategoryDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}
