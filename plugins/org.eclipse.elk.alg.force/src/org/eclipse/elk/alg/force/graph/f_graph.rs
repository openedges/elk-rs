use std::sync::Arc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

use crate::org::eclipse::elk::alg::force::options::ForceOptions;

use super::{FBendpointRef, FEdgeRef, FLabelRef, FNodeRef, FParticle};

#[derive(Clone)]
pub enum FParticleRef {
    Node(FNodeRef),
    Label(FLabelRef),
    Bend(FBendpointRef),
}

impl FParticleRef {
    pub fn ptr_eq(&self, other: &FParticleRef) -> bool {
        match (self, other) {
            (FParticleRef::Node(a), FParticleRef::Node(b)) => Arc::ptr_eq(a, b),
            (FParticleRef::Label(a), FParticleRef::Label(b)) => Arc::ptr_eq(a, b),
            (FParticleRef::Bend(a), FParticleRef::Bend(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }

    pub fn with_particle_mut<R>(&self, f: impl FnOnce(&mut FParticle) -> R) -> Option<R> {
        match self {
            FParticleRef::Node(node) => node
                .lock()
                .ok()
                .map(|mut node_guard| f(node_guard.particle_mut())),
            FParticleRef::Label(label) => label
                .lock()
                .ok()
                .map(|mut label_guard| f(label_guard.particle_mut())),
            FParticleRef::Bend(bend) => bend
                .lock()
                .ok()
                .map(|mut bend_guard| f(bend_guard.particle_mut())),
        }
    }

    pub fn with_particle_ref<R>(&self, f: impl FnOnce(&FParticle) -> R) -> Option<R> {
        match self {
            FParticleRef::Node(node) => node.lock().ok().map(|node_guard| f(node_guard.particle())),
            FParticleRef::Label(label) => label
                .lock()
                .ok()
                .map(|label_guard| f(label_guard.particle())),
            FParticleRef::Bend(bend) => bend.lock().ok().map(|bend_guard| f(bend_guard.particle())),
        }
    }

    pub fn as_node(&self) -> Option<FNodeRef> {
        match self {
            FParticleRef::Node(node) => Some(node.clone()),
            _ => None,
        }
    }

    pub fn as_bendpoint(&self) -> Option<FBendpointRef> {
        match self {
            FParticleRef::Bend(bend) => Some(bend.clone()),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct FGraph {
    properties: MapPropertyHolder,
    nodes: Vec<FNodeRef>,
    edges: Vec<FEdgeRef>,
    labels: Vec<FLabelRef>,
    bendpoints: Vec<FBendpointRef>,
    adjacency: Vec<Vec<i32>>,
}

impl FGraph {
    pub fn new() -> Self {
        FGraph {
            properties: MapPropertyHolder::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            labels: Vec::new(),
            bendpoints: Vec::new(),
            adjacency: Vec::new(),
        }
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

    pub fn nodes(&self) -> &Vec<FNodeRef> {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut Vec<FNodeRef> {
        &mut self.nodes
    }

    pub fn edges(&self) -> &Vec<FEdgeRef> {
        &self.edges
    }

    pub fn edges_mut(&mut self) -> &mut Vec<FEdgeRef> {
        &mut self.edges
    }

    pub fn labels(&self) -> &Vec<FLabelRef> {
        &self.labels
    }

    pub fn labels_mut(&mut self) -> &mut Vec<FLabelRef> {
        &mut self.labels
    }

    pub fn bendpoints(&self) -> &Vec<FBendpointRef> {
        &self.bendpoints
    }

    pub fn bendpoints_mut(&mut self) -> &mut Vec<FBendpointRef> {
        &mut self.bendpoints
    }

    pub fn particles(&self) -> Vec<FParticleRef> {
        let mut particles = Vec::new();
        particles.extend(self.nodes.iter().cloned().map(FParticleRef::Node));
        particles.extend(self.labels.iter().cloned().map(FParticleRef::Label));
        particles.extend(self.bendpoints.iter().cloned().map(FParticleRef::Bend));
        particles
    }

    pub fn get_connection(&self, particle1: &FParticleRef, particle2: &FParticleRef) -> i32 {
        match (particle1, particle2) {
            (FParticleRef::Node(node1), FParticleRef::Node(node2)) => {
                let (id1, id2) = {
                    let node1_guard = node1.lock().ok();
                    let node2_guard = node2.lock().ok();
                    match (node1_guard, node2_guard) {
                        (Some(node1_guard), Some(node2_guard)) => {
                            (node1_guard.id(), node2_guard.id())
                        }
                        _ => return 0,
                    }
                };
                if id1 >= self.adjacency.len() || id2 >= self.adjacency.len() {
                    return 0;
                }
                self.adjacency[id1][id2] + self.adjacency[id2][id1]
            }
            (FParticleRef::Bend(b1), FParticleRef::Bend(b2)) => {
                let edge1 = b1.lock().ok().and_then(|b| b.edge());
                let edge2 = b2.lock().ok().and_then(|b| b.edge());
                match (edge1, edge2) {
                    (Some(edge1), Some(edge2)) if Arc::ptr_eq(&edge1, &edge2) => edge2
                        .lock()
                        .ok()
                        .and_then(|mut edge_guard| edge_guard.get_property(ForceOptions::PRIORITY))
                        .unwrap_or(1),
                    _ => 0,
                }
            }
            _ => 0,
        }
    }

    pub fn calc_adjacency(&mut self) {
        let n = self.nodes.len();
        self.adjacency = vec![vec![0; n]; n];
        for edge in &self.edges {
            let (source_id, target_id, priority) = {
                let edge_guard = edge.lock().ok();
                let Some(mut edge_guard) = edge_guard else {
                    continue;
                };
                let source_id = edge_guard
                    .source()
                    .and_then(|node| node.lock().ok().map(|n| n.id()));
                let target_id = edge_guard
                    .target()
                    .and_then(|node| node.lock().ok().map(|n| n.id()));
                let priority = edge_guard.get_property(ForceOptions::PRIORITY).unwrap_or(1);
                match (source_id, target_id) {
                    (Some(source_id), Some(target_id)) => (source_id, target_id, priority),
                    _ => continue,
                }
            };
            if source_id < n && target_id < n {
                self.adjacency[source_id][target_id] += priority;
            }
        }
    }
}
