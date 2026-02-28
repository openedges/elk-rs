use std::fmt;
use std::sync::{Arc, Weak};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::math::{
    elk_math::ElkMath, kvector::KVector, kvector_chain::KVectorChain,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

use super::{FBendpointRef, FLabelRef, FNodeRef};

pub type FEdgeRef = Arc<Mutex<FEdge>>;

pub struct FEdge {
    properties: MapPropertyHolder,
    bendpoints: Vec<FBendpointRef>,
    labels: Vec<FLabelRef>,
    source: Option<Weak<Mutex<super::FNode>>>,
    target: Option<Weak<Mutex<super::FNode>>>,
}

impl FEdge {
    pub fn new() -> FEdgeRef {
        Arc::new(Mutex::new(FEdge {
            properties: MapPropertyHolder::new(),
            bendpoints: Vec::new(),
            labels: Vec::new(),
            source: None,
            target: None,
        }))
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

    pub fn has_property<T: Clone + Send + Sync + 'static>(&self, property: &Property<T>) -> bool {
        self.properties.has_property(property)
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
        value: Option<T>,
    ) {
        self.properties.set_property(property, value);
    }

    pub fn source(&self) -> Option<FNodeRef> {
        self.source.as_ref().and_then(|node| node.upgrade())
    }

    pub fn target(&self) -> Option<FNodeRef> {
        self.target.as_ref().and_then(|node| node.upgrade())
    }

    pub fn set_source(edge: &FEdgeRef, source: Option<FNodeRef>) {
        if let Ok(mut edge_guard) = edge.lock() {
            edge_guard.source = source.map(|node| Arc::downgrade(&node));
        }
    }

    pub fn set_target(edge: &FEdgeRef, target: Option<FNodeRef>) {
        if let Ok(mut edge_guard) = edge.lock() {
            edge_guard.target = target.map(|node| Arc::downgrade(&node));
        }
    }

    pub fn bendpoints(&self) -> &Vec<FBendpointRef> {
        &self.bendpoints
    }

    pub fn bendpoints_mut(&mut self) -> &mut Vec<FBendpointRef> {
        &mut self.bendpoints
    }

    pub fn labels(&self) -> &Vec<FLabelRef> {
        &self.labels
    }

    pub fn labels_mut(&mut self) -> &mut Vec<FLabelRef> {
        &mut self.labels
    }

    pub fn source_point(&self) -> Option<KVector> {
        let source = self.source()?;
        let target = self.target()?;
        let (source_pos, source_size, target_pos) = {
            let source_guard = source.lock().ok()?;
            let target_guard = target.lock().ok()?;
            (
                *source_guard.position_ref(),
                *source_guard.size_ref(),
                *target_guard.position_ref(),
            )
        };
        let mut v = KVector::from_vector(&target_pos);
        v.sub(&source_pos);
        ElkMath::clip_vector(&mut v, source_size.x, source_size.y);
        v.add(&source_pos);
        Some(v)
    }

    pub fn target_point(&self) -> Option<KVector> {
        let source = self.source()?;
        let target = self.target()?;
        let (source_pos, target_pos, target_size) = {
            let source_guard = source.lock().ok()?;
            let target_guard = target.lock().ok()?;
            (
                *source_guard.position_ref(),
                *target_guard.position_ref(),
                *target_guard.size_ref(),
            )
        };
        let mut v = KVector::from_vector(&source_pos);
        v.sub(&target_pos);
        ElkMath::clip_vector(&mut v, target_size.x, target_size.y);
        v.add(&target_pos);
        Some(v)
    }

    pub fn to_vector_chain(&self) -> Option<KVectorChain> {
        let mut chain = KVectorChain::new();
        let source = self.source_point()?;
        let target = self.target_point()?;
        chain.add_vector(source);
        for bend in &self.bendpoints {
            if let Ok(bend_guard) = bend.lock() {
                chain.add_vector(*bend_guard.position_ref());
            }
        }
        chain.add_vector(target);
        Some(chain)
    }

    pub fn distribute_bendpoints(&mut self) {
        let count = self.bendpoints.len();
        if count == 0 {
            return;
        }
        let (source_pos, target_pos) = match (self.source(), self.target()) {
            (Some(source), Some(target)) => {
                let source_guard = source.lock().ok();
                let target_guard = target.lock().ok();
                match (source_guard, target_guard) {
                    (Some(source_guard), Some(target_guard)) => {
                        (*source_guard.position_ref(), *target_guard.position_ref())
                    }
                    _ => return,
                }
            }
            _ => return,
        };

        let mut incr = KVector::from_vector(&target_pos);
        incr.sub(&source_pos);
        incr.scale(1.0 / ((count + 1) as f64));
        let mut pos = KVector::from_vector(&source_pos);
        for bend in &self.bendpoints {
            if let Ok(mut bend_guard) = bend.lock() {
                bend_guard.position().x = pos.x + incr.x;
                bend_guard.position().y = pos.y + incr.y;
                pos.add(&incr);
            }
        }
    }
}

impl fmt::Display for FEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source_label = self
            .source()
            .and_then(|node| node.lock().ok().map(|node_guard| node_guard.to_string()));
        let target_label = self
            .target()
            .and_then(|node| node.lock().ok().map(|node_guard| node_guard.to_string()));
        match (source_label, target_label) {
            (Some(source_label), Some(target_label)) => {
                write!(f, "{}->{}", source_label, target_label)
            }
            _ => write!(f, "e_{:p}", self),
        }
    }
}

impl Default for FEdge {
    fn default() -> Self {
        FEdge {
            properties: MapPropertyHolder::new(),
            bendpoints: Vec::new(),
            labels: Vec::new(),
            source: None,
            target: None,
        }
    }
}
