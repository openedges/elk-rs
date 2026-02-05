use std::sync::{Arc, Mutex};

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;

use super::{DCComponent, DCComponentRef, DCComponentWeak, DCElementRef};

#[derive(Clone)]
pub struct DCGraph {
    properties: MapPropertyHolder,
    components: Vec<DCComponentRef>,
    dimensions: KVector,
    inset: f64,
}

impl DCGraph {
    pub fn new(components: Vec<Vec<DCElementRef>>, inset: f64) -> Self {
        let mut graph_components: Vec<DCComponentRef> = Vec::new();
        for elements in components {
            let component = Arc::new(Mutex::new(DCComponent::new()));
            let weak: DCComponentWeak = Arc::downgrade(&component);
            for elem in elements {
                if let Ok(mut elem_guard) = elem.lock() {
                    elem_guard.set_component(weak.clone());
                }
                if let Ok(mut comp_guard) = component.lock() {
                    comp_guard.add_element(elem.clone());
                }
            }
            graph_components.push(component);
        }

        DCGraph {
            properties: MapPropertyHolder::new(),
            components: graph_components,
            dimensions: KVector::new(),
            inset,
        }
    }

    pub fn components(&self) -> &Vec<DCComponentRef> {
        &self.components
    }

    pub fn get_dimensions(&self) -> KVector {
        self.dimensions
    }

    pub fn set_dimensions(&mut self, dimensions: KVector) {
        self.dimensions = dimensions;
    }

    pub fn get_inset(&self) -> f64 {
        self.inset
    }

    pub fn properties(&self) -> &MapPropertyHolder {
        &self.properties
    }

    pub fn properties_mut(&mut self) -> &mut MapPropertyHolder {
        &mut self.properties
    }

    pub fn copy_properties(&mut self, other: &MapPropertyHolder) {
        self.properties.copy_properties(other);
    }
}
