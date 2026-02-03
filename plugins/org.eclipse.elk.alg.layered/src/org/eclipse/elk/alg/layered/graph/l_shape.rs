use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use super::LGraphElement;

pub struct LShape {
    element: LGraphElement,
    position: KVector,
    size: KVector,
}

impl LShape {
    pub fn new() -> Self {
        LShape {
            element: LGraphElement::new(),
            position: KVector::new(),
            size: KVector::new(),
        }
    }

    pub fn graph_element(&mut self) -> &mut LGraphElement {
        &mut self.element
    }

    pub fn position(&mut self) -> &mut KVector {
        &mut self.position
    }

    pub fn position_ref(&self) -> &KVector {
        &self.position
    }

    pub fn size(&mut self) -> &mut KVector {
        &mut self.size
    }

    pub fn size_ref(&self) -> &KVector {
        &self.size
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
    ) -> Option<T> {
        self.element.get_property(property)
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
        value: Option<T>,
    ) {
        self.element.set_property(property, value);
    }
}

impl Default for LShape {
    fn default() -> Self {
        Self::new()
    }
}
