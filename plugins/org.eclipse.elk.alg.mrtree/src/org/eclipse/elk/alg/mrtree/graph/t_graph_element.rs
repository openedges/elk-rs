use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

pub struct TGraphElement {
    pub id: i32,
    properties: MapPropertyHolder,
}

impl TGraphElement {
    pub fn new(id: i32) -> Self {
        TGraphElement {
            id,
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
        &self,
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
}

impl Default for TGraphElement {
    fn default() -> Self {
        Self::new(0)
    }
}
