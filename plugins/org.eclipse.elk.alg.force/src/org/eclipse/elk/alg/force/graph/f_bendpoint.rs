use std::fmt;
use std::sync::{Arc, Weak};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

use super::{FEdgeRef, FParticle};

pub type FBendpointRef = Arc<Mutex<FBendpoint>>;

#[derive(Default)]
pub struct FBendpoint {
    particle: FParticle,
    edge: Option<Weak<Mutex<super::FEdge>>>,
}

impl FBendpoint {
    pub fn new(edge: &FEdgeRef) -> FBendpointRef {
        let bend = Arc::new(Mutex::new(FBendpoint {
            particle: FParticle::new(),
            edge: Some(Arc::downgrade(edge)),
        }));
        {
            let mut edge_guard = edge.lock();
            edge_guard.bendpoints_mut().push(bend.clone());
        }
        bend
    }

    pub fn particle(&self) -> &FParticle {
        &self.particle
    }

    pub fn particle_mut(&mut self) -> &mut FParticle {
        &mut self.particle
    }

    pub fn properties(&self) -> &MapPropertyHolder {
        self.particle.properties()
    }

    pub fn properties_mut(&mut self) -> &mut MapPropertyHolder {
        self.particle.properties_mut()
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
    ) -> Option<T> {
        self.particle.get_property(property)
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
        value: Option<T>,
    ) {
        self.particle.set_property(property, value);
    }

    pub fn position(&mut self) -> &mut KVector {
        self.particle.position()
    }

    pub fn position_ref(&self) -> &KVector {
        self.particle.position_ref()
    }

    pub fn size(&mut self) -> &mut KVector {
        self.particle.size()
    }

    pub fn size_ref(&self) -> &KVector {
        self.particle.size_ref()
    }

    pub fn edge(&self) -> Option<FEdgeRef> {
        self.edge.as_ref().and_then(|edge| edge.upgrade())
    }
}

impl fmt::Display for FBendpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(edge) = self.edge() {
            {
                let edge_guard = edge.lock();
                return write!(f, "b[{}]", edge_guard);
            }
        }
        write!(f, "b_{:p}", self)
    }
}
