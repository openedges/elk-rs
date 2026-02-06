use std::fmt;
use std::sync::{Arc, Mutex, Weak};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

use super::FParticle;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;

pub type FNodeRef = Arc<Mutex<FNode>>;

#[derive(Default)]
pub struct FNode {
    particle: FParticle,
    id: usize,
    label: Option<String>,
    parent: Option<Weak<Mutex<FNode>>>,
    children: Vec<FNodeRef>,
}

impl FNode {
    pub fn new() -> FNodeRef {
        Arc::new(Mutex::new(FNode {
            particle: FParticle::new(),
            id: 0,
            label: None,
            parent: None,
            children: Vec::new(),
        }))
    }

    pub fn new_with_label(label: impl Into<String>) -> FNodeRef {
        let node = Self::new();
        if let Ok(mut node_guard) = node.lock() {
            node_guard.label = Some(label.into());
        }
        node
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

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = Some(label.into());
    }

    pub fn parent(&self) -> Option<FNodeRef> {
        self.parent.as_ref().and_then(|parent| parent.upgrade())
    }

    pub fn set_parent(&mut self, parent: Option<FNodeRef>) {
        self.parent = parent.map(|node| Arc::downgrade(&node));
    }

    pub fn children(&self) -> &Vec<FNodeRef> {
        &self.children
    }

    pub fn children_mut(&mut self) -> &mut Vec<FNodeRef> {
        &mut self.children
    }

    pub fn is_compound(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn depth(&self) -> usize {
        let mut depth = 0;
        let mut current = self.parent();
        while let Some(node) = current {
            depth += 1;
            current = node.lock().ok().and_then(|node_guard| node_guard.parent());
        }
        depth
    }

}

impl fmt::Display for FNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(label) = self.label.as_deref() {
            if !label.is_empty() {
                return write!(f, "n_{}", label);
            }
        }
        write!(f, "n_{}", self.id)
    }
}
