use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use super::LShape;

pub struct LLabel {
    shape: LShape,
    text: String,
}

impl LLabel {
    pub fn new() -> Self {
        LLabel::with_text("")
    }

    pub fn with_text(text: impl Into<String>) -> Self {
        LLabel {
            shape: LShape::new(),
            text: text.into(),
        }
    }

    pub fn shape(&mut self) -> &mut LShape {
        &mut self.shape
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
    ) -> Option<T> {
        self.shape.get_property(property)
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
        value: Option<T>,
    ) {
        self.shape.set_property(property, value);
    }

    pub fn get_designation(&mut self) -> Option<String> {
        if !self.text.is_empty() {
            return Some(self.text.clone());
        }
        self.shape.graph_element().get_designation()
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&mut self) -> String {
        if let Some(designation) = self.get_designation() {
            format!("l_{designation}")
        } else {
            "label".to_owned()
        }
    }
}

impl Default for LLabel {
    fn default() -> Self {
        Self::new()
    }
}
