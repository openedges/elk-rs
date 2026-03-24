use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

// --- Typed index types ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FNodeId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FEdgeId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FLabelId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FBendpointId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FParticleId {
    Node(FNodeId),
    Label(FLabelId),
    Bend(FBendpointId),
}

// --- Arena struct (SoA layout) ---

pub struct FArena {
    // Node storage
    pub node_position: Vec<KVector>,
    pub node_size: Vec<KVector>,
    pub node_displacement: Vec<KVector>,
    pub node_properties: Vec<MapPropertyHolder>,
    pub node_id: Vec<usize>,
    pub node_label: Vec<Option<String>>,
    pub node_parent: Vec<Option<FNodeId>>,
    pub node_children: Vec<Vec<FNodeId>>,

    // Edge storage
    pub edge_properties: Vec<MapPropertyHolder>,
    pub edge_source: Vec<Option<FNodeId>>,
    pub edge_target: Vec<Option<FNodeId>>,
    pub edge_bendpoints: Vec<Vec<FBendpointId>>,
    pub edge_labels: Vec<Vec<FLabelId>>,

    // Label storage
    pub label_position: Vec<KVector>,
    pub label_size: Vec<KVector>,
    pub label_displacement: Vec<KVector>,
    pub label_properties: Vec<MapPropertyHolder>,
    pub label_text: Vec<Option<String>>,
    pub label_edge: Vec<Option<FEdgeId>>,

    // Bendpoint storage
    pub bend_position: Vec<KVector>,
    pub bend_size: Vec<KVector>,
    pub bend_displacement: Vec<KVector>,
    pub bend_properties: Vec<MapPropertyHolder>,
    pub bend_edge: Vec<Option<FEdgeId>>,
}

impl FArena {
    pub fn new() -> Self {
        FArena {
            node_position: Vec::new(),
            node_size: Vec::new(),
            node_displacement: Vec::new(),
            node_properties: Vec::new(),
            node_id: Vec::new(),
            node_label: Vec::new(),
            node_parent: Vec::new(),
            node_children: Vec::new(),

            edge_properties: Vec::new(),
            edge_source: Vec::new(),
            edge_target: Vec::new(),
            edge_bendpoints: Vec::new(),
            edge_labels: Vec::new(),

            label_position: Vec::new(),
            label_size: Vec::new(),
            label_displacement: Vec::new(),
            label_properties: Vec::new(),
            label_text: Vec::new(),
            label_edge: Vec::new(),

            bend_position: Vec::new(),
            bend_size: Vec::new(),
            bend_displacement: Vec::new(),
            bend_properties: Vec::new(),
            bend_edge: Vec::new(),
        }
    }

    // --- Node builder ---

    pub fn add_node(&mut self) -> FNodeId {
        let id = FNodeId(self.node_position.len());
        self.node_position.push(KVector::new());
        self.node_size.push(KVector::new());
        self.node_displacement.push(KVector::new());
        self.node_properties.push(MapPropertyHolder::new());
        self.node_id.push(id.0);
        self.node_label.push(None);
        self.node_parent.push(None);
        self.node_children.push(Vec::new());
        id
    }

    pub fn node_count(&self) -> usize {
        self.node_position.len()
    }

    // --- Edge builder ---

    pub fn add_edge(&mut self) -> FEdgeId {
        let id = FEdgeId(self.edge_properties.len());
        self.edge_properties.push(MapPropertyHolder::new());
        self.edge_source.push(None);
        self.edge_target.push(None);
        self.edge_bendpoints.push(Vec::new());
        self.edge_labels.push(Vec::new());
        id
    }

    pub fn set_edge_source(&mut self, edge: FEdgeId, node: FNodeId) {
        self.edge_source[edge.0] = Some(node);
    }

    pub fn set_edge_target(&mut self, edge: FEdgeId, node: FNodeId) {
        self.edge_target[edge.0] = Some(node);
    }

    pub fn edge_count(&self) -> usize {
        self.edge_properties.len()
    }

    // --- Label builder ---

    pub fn add_label(&mut self, edge: FEdgeId) -> FLabelId {
        let id = FLabelId(self.label_position.len());
        self.label_position.push(KVector::new());
        self.label_size.push(KVector::new());
        self.label_displacement.push(KVector::new());
        self.label_properties.push(MapPropertyHolder::new());
        self.label_text.push(None);
        self.label_edge.push(Some(edge));
        self.edge_labels[edge.0].push(id);
        id
    }

    pub fn label_count(&self) -> usize {
        self.label_position.len()
    }

    // --- Bendpoint builder ---

    pub fn add_bendpoint(&mut self, edge: FEdgeId) -> FBendpointId {
        let id = FBendpointId(self.bend_position.len());
        self.bend_position.push(KVector::new());
        self.bend_size.push(KVector::new());
        self.bend_displacement.push(KVector::new());
        self.bend_properties.push(MapPropertyHolder::new());
        self.bend_edge.push(Some(edge));
        self.edge_bendpoints[edge.0].push(id);
        id
    }

    pub fn bend_count(&self) -> usize {
        self.bend_position.len()
    }

    // --- Particle access helpers ---

    pub fn particle_ids(&self) -> Vec<FParticleId> {
        let mut ids = Vec::with_capacity(
            self.node_count() + self.label_count() + self.bend_count(),
        );
        for i in 0..self.node_count() {
            ids.push(FParticleId::Node(FNodeId(i)));
        }
        for i in 0..self.label_count() {
            ids.push(FParticleId::Label(FLabelId(i)));
        }
        for i in 0..self.bend_count() {
            ids.push(FParticleId::Bend(FBendpointId(i)));
        }
        ids
    }

    pub fn particle_position(&self, id: FParticleId) -> &KVector {
        match id {
            FParticleId::Node(n) => &self.node_position[n.0],
            FParticleId::Label(l) => &self.label_position[l.0],
            FParticleId::Bend(b) => &self.bend_position[b.0],
        }
    }

    pub fn particle_position_mut(&mut self, id: FParticleId) -> &mut KVector {
        match id {
            FParticleId::Node(n) => &mut self.node_position[n.0],
            FParticleId::Label(l) => &mut self.label_position[l.0],
            FParticleId::Bend(b) => &mut self.bend_position[b.0],
        }
    }

    pub fn particle_size(&self, id: FParticleId) -> &KVector {
        match id {
            FParticleId::Node(n) => &self.node_size[n.0],
            FParticleId::Label(l) => &self.label_size[l.0],
            FParticleId::Bend(b) => &self.bend_size[b.0],
        }
    }

    pub fn particle_displacement(&self, id: FParticleId) -> &KVector {
        match id {
            FParticleId::Node(n) => &self.node_displacement[n.0],
            FParticleId::Label(l) => &self.label_displacement[l.0],
            FParticleId::Bend(b) => &self.bend_displacement[b.0],
        }
    }

    pub fn particle_displacement_mut(&mut self, id: FParticleId) -> &mut KVector {
        match id {
            FParticleId::Node(n) => &mut self.node_displacement[n.0],
            FParticleId::Label(l) => &mut self.label_displacement[l.0],
            FParticleId::Bend(b) => &mut self.bend_displacement[b.0],
        }
    }

    pub fn particle_properties(&self, id: FParticleId) -> &MapPropertyHolder {
        match id {
            FParticleId::Node(n) => &self.node_properties[n.0],
            FParticleId::Label(l) => &self.label_properties[l.0],
            FParticleId::Bend(b) => &self.bend_properties[b.0],
        }
    }

    pub fn particle_properties_mut(&mut self, id: FParticleId) -> &mut MapPropertyHolder {
        match id {
            FParticleId::Node(n) => &mut self.node_properties[n.0],
            FParticleId::Label(l) => &mut self.label_properties[l.0],
            FParticleId::Bend(b) => &mut self.bend_properties[b.0],
        }
    }

    pub fn particle_get_property<T: Clone + Send + Sync + 'static>(
        &self,
        id: FParticleId,
        property: &Property<T>,
    ) -> Option<T> {
        self.particle_properties(id).get_property(property)
    }

    /// Radius of the particle (half-diagonal of size).
    pub fn particle_radius(&self, id: FParticleId) -> f64 {
        let size = self.particle_size(id);
        f64::sqrt(size.x * size.x + size.y * size.y) / 2.0
    }

    // --- Connection helpers ---

    /// Get the connection weight between two particles (for force models).
    pub fn get_connection(&self, p1: FParticleId, p2: FParticleId) -> i32 {
        match (p1, p2) {
            (FParticleId::Node(n1), FParticleId::Node(n2)) => {
                let id1 = self.node_id[n1.0];
                let id2 = self.node_id[n2.0];
                if id1 >= self.node_count() || id2 >= self.node_count() {
                    return 0;
                }
                // Requires adjacency to be pre-computed
                0 // placeholder — adjacency stored in FGraph
            }
            (FParticleId::Bend(b1), FParticleId::Bend(b2)) => {
                let e1 = self.bend_edge[b1.0];
                let e2 = self.bend_edge[b2.0];
                match (e1, e2) {
                    (Some(e1), Some(e2)) if e1 == e2 => {
                        self.edge_properties[e1.0]
                            .get_property(
                                crate::org::eclipse::elk::alg::force::options::ForceOptions::PRIORITY,
                            )
                            .unwrap_or(1)
                    }
                    _ => 0,
                }
            }
            _ => 0,
        }
    }
}

impl Default for FArena {
    fn default() -> Self {
        Self::new()
    }
}
