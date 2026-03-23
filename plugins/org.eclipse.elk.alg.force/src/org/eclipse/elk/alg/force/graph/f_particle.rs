use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

#[derive(Clone)]
pub struct FParticle {
    properties: MapPropertyHolder,
    displacement: KVector,
    position: KVector,
    size: KVector,
}

impl FParticle {
    pub fn new() -> Self {
        FParticle {
            properties: MapPropertyHolder::new(),
            displacement: KVector::new(),
            position: KVector::new(),
            size: KVector::new(),
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

    pub fn displacement(&mut self) -> &mut KVector {
        &mut self.displacement
    }

    pub fn displacement_ref(&self) -> &KVector {
        &self.displacement
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

    pub fn radius(&self) -> f64 {
        self.size.length() / 2.0
    }
}

impl Default for FParticle {
    fn default() -> Self {
        Self::new()
    }
}
