use org_eclipse_elk_core::org::eclipse::elk::core::math::{
    elk_math::ElkMath, elk_rectangle::ElkRectangle, kvector::KVector, kvector_chain::KVectorChain,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;

use super::{DCComponentWeak, DCExtension};

pub struct DCElement {
    shape: KVectorChain,
    bounds: ElkRectangle,
    component: Option<DCComponentWeak>,
    coords: Option<Vec<f64>>,
    parent_coords: Option<KVector>,
    extensions: Vec<DCExtension>,
    properties: MapPropertyHolder,
}

impl DCElement {
    pub fn new(poly_path: KVectorChain) -> Self {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for v in poly_path.iter() {
            min_x = min_x.min(v.x);
            min_y = min_y.min(v.y);
            max_x = max_x.max(v.x);
            max_y = max_y.max(v.y);
        }
        let bounds = ElkRectangle::with_values(min_x, min_y, max_x - min_x, max_y - min_y);
        DCElement {
            shape: poly_path,
            bounds,
            component: None,
            coords: None,
            parent_coords: None,
            extensions: Vec::new(),
            properties: MapPropertyHolder::new(),
        }
    }

    pub fn get_offset(&self) -> KVector {
        let component = self
            .component
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .expect("DCElement without component");
        let component_guard = component.lock().expect("component lock");
        component_guard.get_offset()
    }

    pub fn get_coords(&mut self) -> Vec<f64> {
        if let Some(coords) = &self.coords {
            return coords.clone();
        }
        let mut coords = Vec::with_capacity(self.shape.len() * 2);
        for c in self.shape.iter() {
            coords.push(c.x);
            coords.push(c.y);
        }
        self.coords = Some(coords.clone());
        coords
    }

    pub fn add_extension(&mut self, extension: DCExtension) {
        self.extensions.push(extension);
    }

    pub fn get_extensions(&self) -> &Vec<DCExtension> {
        &self.extensions
    }

    pub fn get_bounds(&self) -> ElkRectangle {
        self.bounds
    }

    pub fn set_parent_coords(&mut self, coords: KVector) {
        self.parent_coords = Some(coords);
    }

    pub fn get_parent_coords(&self) -> Option<KVector> {
        self.parent_coords
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

    pub(crate) fn set_component(&mut self, comp: DCComponentWeak) {
        self.component = Some(comp);
    }

    pub(crate) fn intersects(&self, rect: &ElkRectangle) -> bool {
        ElkMath::intersects((rect, &self.shape)) || ElkMath::contains((rect, &self.shape))
    }
}
