use std::fmt;
use std::sync::{Arc, Weak};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

use crate::org::eclipse::elk::alg::force::options::ForceOptions;

use super::{FEdgeRef, FParticle};

pub type FLabelRef = Arc<Mutex<FLabel>>;

#[derive(Default)]
pub struct FLabel {
    particle: FParticle,
    edge: Option<Weak<Mutex<super::FEdge>>>,
    text: Option<String>,
}

impl FLabel {
    pub fn new(edge: &FEdgeRef, text: impl Into<String>) -> FLabelRef {
        let label = Arc::new(Mutex::new(FLabel {
            particle: FParticle::new(),
            edge: Some(Arc::downgrade(edge)),
            text: Some(text.into()),
        }));
        if let Ok(mut edge_guard) = edge.lock() {
            edge_guard.labels_mut().push(label.clone());
        }
        label
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

    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn edge(&self) -> Option<FEdgeRef> {
        self.edge.as_ref().and_then(|edge| edge.upgrade())
    }

    pub fn refresh_position(&mut self) {
        let Some(edge) = self.edge() else { return };
        let place_inline = self
            .get_property(ForceOptions::EDGE_LABELS_INLINE)
            .unwrap_or(false);
        let (src, tgt) = {
            let edge_guard = edge.lock().ok();
            let Some(edge_guard) = edge_guard else { return };
            let source = edge_guard
                .source()
                .and_then(|node| node.lock().ok().map(|n| *n.position_ref()));
            let target = edge_guard
                .target()
                .and_then(|node| node.lock().ok().map(|n| *n.position_ref()));
            match (source, target) {
                (Some(source), Some(target)) => (source, target),
                _ => return,
            }
        };

        let size = *self.size_ref();
        if place_inline {
            let mut src_to_tgt = KVector::from_vector(&tgt);
            src_to_tgt.sub(&src).scale(0.5);
            let mut to_label_center = KVector::from_vector(&size);
            to_label_center.scale(0.5);
            let mut new_pos = KVector::from_vector(&src);
            new_pos.add(&src_to_tgt).sub(&to_label_center);
            self.position().set(&new_pos);
            return;
        }

        let spacing = edge
            .lock()
            .ok()
            .and_then(|mut edge_guard| edge_guard.get_property(ForceOptions::SPACING_EDGE_LABEL))
            .unwrap_or(0.0);
        let pos = self.position();
        if src.x >= tgt.x {
            if src.y >= tgt.y {
                pos.x = tgt.x + ((src.x - tgt.x) / 2.0) + spacing;
                pos.y = tgt.y + ((src.y - tgt.y) / 2.0) - spacing - size.y;
            } else {
                pos.x = tgt.x + ((src.x - tgt.x) / 2.0) + spacing;
                pos.y = src.y + ((tgt.y - src.y) / 2.0) + spacing;
            }
        } else if src.y >= tgt.y {
            pos.x = src.x + ((tgt.x - src.x) / 2.0) + spacing;
            pos.y = tgt.y + ((src.y - tgt.y) / 2.0) + spacing;
        } else {
            pos.x = src.x + ((tgt.x - src.x) / 2.0) + spacing;
            pos.y = src.y + ((tgt.y - src.y) / 2.0) - spacing - size.y;
        }
    }
}

impl fmt::Display for FLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(text) = self.text.as_deref() {
            if !text.is_empty() {
                return write!(f, "l_{}", text);
            }
        }
        let edge = self.edge();
        if let Some(edge) = edge {
            let edge_guard = edge.lock().ok();
            if let Some(edge_guard) = edge_guard {
                return write!(f, "l[{}]", edge_guard);
            }
        }
        write!(f, "l_{:p}", self)
    }
}
