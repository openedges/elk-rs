use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

pub struct LGraphElement {
    pub id: i32,
    properties: MapPropertyHolder,
}

impl LGraphElement {
    pub fn new() -> Self {
        LGraphElement {
            id: 0,
            properties: MapPropertyHolder::new(),
        }
    }

    pub fn properties(&self) -> &MapPropertyHolder {
        &self.properties
    }

    pub fn properties_mut(&mut self) -> &mut MapPropertyHolder {
        &mut self.properties
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
    ) -> Option<T> {
        self.properties.get_property(property)
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
        value: Option<T>,
    ) {
        self.properties.set_property(property, value);
    }

    pub fn get_designation(&self) -> Option<String> {
        None
    }
}

impl Default for LGraphElement {
    fn default() -> Self {
        Self::new()
    }
}
