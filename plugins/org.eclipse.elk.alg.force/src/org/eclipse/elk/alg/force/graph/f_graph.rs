use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{
    elk_math::ElkMath, kvector_chain::KVectorChain,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

use crate::org::eclipse::elk::alg::force::options::ForceOptions;

use super::{FArena, FBendpointId, FEdgeId, FLabelId, FNodeId, FParticleId};

pub struct FGraph {
    pub arena: FArena,
    pub nodes: Vec<FNodeId>,
    pub edges: Vec<FEdgeId>,
    pub labels: Vec<FLabelId>,
    pub bendpoints: Vec<FBendpointId>,
    pub properties: MapPropertyHolder,
    pub adjacency: Vec<Vec<i32>>,
}

impl FGraph {
    pub fn new() -> Self {
        FGraph {
            arena: FArena::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            labels: Vec::new(),
            bendpoints: Vec::new(),
            properties: MapPropertyHolder::new(),
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

    pub fn particle_ids(&self) -> Vec<FParticleId> {
        let mut ids = Vec::with_capacity(
            self.nodes.len() + self.labels.len() + self.bendpoints.len(),
        );
        for &n in &self.nodes {
            ids.push(FParticleId::Node(n));
        }
        for &l in &self.labels {
            ids.push(FParticleId::Label(l));
        }
        for &b in &self.bendpoints {
            ids.push(FParticleId::Bend(b));
        }
        ids
    }

    pub fn get_connection(&self, p1: FParticleId, p2: FParticleId) -> i32 {
        match (p1, p2) {
            (FParticleId::Node(n1), FParticleId::Node(n2)) => {
                let id1 = self.arena.node_id[n1.0];
                let id2 = self.arena.node_id[n2.0];
                if id1 >= self.adjacency.len() || id2 >= self.adjacency.len() {
                    return 0;
                }
                self.adjacency[id1][id2] + self.adjacency[id2][id1]
            }
            (FParticleId::Bend(b1), FParticleId::Bend(b2)) => {
                let e1 = self.arena.bend_edge[b1.0];
                let e2 = self.arena.bend_edge[b2.0];
                match (e1, e2) {
                    (Some(e1), Some(e2)) if e1 == e2 => self.arena.edge_properties[e1.0]
                        .get_property(ForceOptions::PRIORITY)
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
        for &eid in &self.edges {
            let source_id = self.arena.edge_source[eid.0].map(|nid| self.arena.node_id[nid.0]);
            let target_id = self.arena.edge_target[eid.0].map(|nid| self.arena.node_id[nid.0]);
            let priority = self.arena.edge_properties[eid.0]
                .get_property(ForceOptions::PRIORITY)
                .unwrap_or(1);
            if let (Some(sid), Some(tid)) = (source_id, target_id) {
                if sid < n && tid < n {
                    self.adjacency[sid][tid] += priority;
                }
            }
        }
    }

    // --- Edge geometry helpers (previously on FEdge) ---

    pub fn edge_source_point(&self, eid: FEdgeId) -> Option<KVector> {
        let src_nid = self.arena.edge_source[eid.0]?;
        let tgt_nid = self.arena.edge_target[eid.0]?;
        let source_pos = self.arena.node_position[src_nid.0];
        let source_size = self.arena.node_size[src_nid.0];
        let target_pos = self.arena.node_position[tgt_nid.0];
        let mut v = KVector::from_vector(&target_pos);
        v.sub(&source_pos);
        ElkMath::clip_vector(&mut v, source_size.x, source_size.y);
        v.add(&source_pos);
        Some(v)
    }

    pub fn edge_target_point(&self, eid: FEdgeId) -> Option<KVector> {
        let src_nid = self.arena.edge_source[eid.0]?;
        let tgt_nid = self.arena.edge_target[eid.0]?;
        let source_pos = self.arena.node_position[src_nid.0];
        let target_pos = self.arena.node_position[tgt_nid.0];
        let target_size = self.arena.node_size[tgt_nid.0];
        let mut v = KVector::from_vector(&source_pos);
        v.sub(&target_pos);
        ElkMath::clip_vector(&mut v, target_size.x, target_size.y);
        v.add(&target_pos);
        Some(v)
    }

    pub fn edge_to_vector_chain(&self, eid: FEdgeId) -> Option<KVectorChain> {
        let mut chain = KVectorChain::new();
        let source = self.edge_source_point(eid)?;
        let target = self.edge_target_point(eid)?;
        chain.add_vector(source);
        for &bid in &self.arena.edge_bendpoints[eid.0] {
            chain.add_vector(self.arena.bend_position[bid.0]);
        }
        chain.add_vector(target);
        Some(chain)
    }

    pub fn distribute_bendpoints(&mut self, eid: FEdgeId) {
        let bends = &self.arena.edge_bendpoints[eid.0];
        let count = bends.len();
        if count == 0 {
            return;
        }
        let src_nid = self.arena.edge_source[eid.0];
        let tgt_nid = self.arena.edge_target[eid.0];
        let (source_pos, target_pos) = match (src_nid, tgt_nid) {
            (Some(s), Some(t)) => (self.arena.node_position[s.0], self.arena.node_position[t.0]),
            _ => return,
        };

        let mut incr = KVector::from_vector(&target_pos);
        incr.sub(&source_pos);
        incr.scale(1.0 / ((count + 1) as f64));
        let mut pos = KVector::from_vector(&source_pos);
        let bend_ids: Vec<FBendpointId> = bends.clone();
        for bid in bend_ids {
            self.arena.bend_position[bid.0].x = pos.x + incr.x;
            self.arena.bend_position[bid.0].y = pos.y + incr.y;
            pos.add(&incr);
        }
    }

    /// Refresh label position (previously FLabel::refresh_position)
    pub fn refresh_label_position(&mut self, lid: FLabelId) {
        let Some(eid) = self.arena.label_edge[lid.0] else {
            return;
        };
        let place_inline = self.arena.label_properties[lid.0]
            .get_property(ForceOptions::EDGE_LABELS_INLINE)
            .unwrap_or(false);

        let src_nid = self.arena.edge_source[eid.0];
        let tgt_nid = self.arena.edge_target[eid.0];
        let (src, tgt) = match (src_nid, tgt_nid) {
            (Some(s), Some(t)) => (self.arena.node_position[s.0], self.arena.node_position[t.0]),
            _ => return,
        };

        let size = self.arena.label_size[lid.0];
        if place_inline {
            let mut src_to_tgt = KVector::from_vector(&tgt);
            src_to_tgt.sub(&src).scale(0.5);
            let mut to_label_center = KVector::from_vector(&size);
            to_label_center.scale(0.5);
            let mut new_pos = KVector::from_vector(&src);
            new_pos.add(&src_to_tgt).sub(&to_label_center);
            self.arena.label_position[lid.0].set(&new_pos);
            return;
        }

        let spacing = self.arena.edge_properties[eid.0]
            .get_property(ForceOptions::SPACING_EDGE_LABEL)
            .unwrap_or(0.0);
        let pos = &mut self.arena.label_position[lid.0];
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

impl Default for FGraph {
    fn default() -> Self {
        Self::new()
    }
}
