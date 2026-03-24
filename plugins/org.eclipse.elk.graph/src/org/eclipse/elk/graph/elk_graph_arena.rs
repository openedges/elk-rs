use crate::org::eclipse::elk::graph::properties::MapPropertyHolder;

// --- Typed index types ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ENodeId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EPortId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EEdgeId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ELabelId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ESectionId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EBendId(pub u32);

/// A connectable shape is either a node or a port (matches ElkConnectableShapeRef).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EConnectableId {
    Node(ENodeId),
    Port(EPortId),
}

/// Label parent element (matches ElkGraphElementRef variants that own labels).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ELabelParent {
    Node(ENodeId),
    Port(EPortId),
    Edge(EEdgeId),
}

impl ENodeId {
    pub const NONE: Self = Self(u32::MAX);
    #[inline]
    pub fn is_none(self) -> bool { self.0 == u32::MAX }
    #[inline]
    pub fn idx(self) -> usize { self.0 as usize }
}

impl EPortId {
    pub const NONE: Self = Self(u32::MAX);
    #[inline]
    pub fn idx(self) -> usize { self.0 as usize }
}

impl EEdgeId {
    #[inline]
    pub fn idx(self) -> usize { self.0 as usize }
}

impl ELabelId {
    #[inline]
    pub fn idx(self) -> usize { self.0 as usize }
}

impl ESectionId {
    #[inline]
    pub fn idx(self) -> usize { self.0 as usize }
}

impl EBendId {
    #[inline]
    pub fn idx(self) -> usize { self.0 as usize }
}

// --- Arena struct (SoA layout, mutable adjacency via Vec<Vec<Id>>) ---

pub struct ElkGraphArena {
    // ── Node attributes (indexed by ENodeId) ──
    pub node_x: Vec<f64>,
    pub node_y: Vec<f64>,
    pub node_width: Vec<f64>,
    pub node_height: Vec<f64>,
    pub node_identifier: Vec<Option<String>>,
    pub node_properties: Vec<MapPropertyHolder>,
    pub node_parent: Vec<Option<ENodeId>>,
    // Node adjacency (mutable Vec<Vec<Id>> — not CSR, supports mutation)
    pub node_children: Vec<Vec<ENodeId>>,
    pub node_ports: Vec<Vec<EPortId>>,
    pub node_contained_edges: Vec<Vec<EEdgeId>>,
    pub node_labels: Vec<Vec<ELabelId>>,
    pub node_incoming_edges: Vec<Vec<EEdgeId>>,
    pub node_outgoing_edges: Vec<Vec<EEdgeId>>,

    // ── Port attributes (indexed by EPortId) ──
    pub port_x: Vec<f64>,
    pub port_y: Vec<f64>,
    pub port_width: Vec<f64>,
    pub port_height: Vec<f64>,
    pub port_identifier: Vec<Option<String>>,
    pub port_properties: Vec<MapPropertyHolder>,
    pub port_owner: Vec<ENodeId>,
    pub port_labels: Vec<Vec<ELabelId>>,
    pub port_incoming_edges: Vec<Vec<EEdgeId>>,
    pub port_outgoing_edges: Vec<Vec<EEdgeId>>,

    // ── Edge attributes (indexed by EEdgeId) ──
    pub edge_identifier: Vec<Option<String>>,
    pub edge_properties: Vec<MapPropertyHolder>,
    pub edge_containing_node: Vec<Option<ENodeId>>,
    pub edge_sources: Vec<Vec<EConnectableId>>,
    pub edge_targets: Vec<Vec<EConnectableId>>,
    pub edge_sections: Vec<Vec<ESectionId>>,
    pub edge_labels: Vec<Vec<ELabelId>>,

    // ── Label attributes (indexed by ELabelId) ──
    pub label_x: Vec<f64>,
    pub label_y: Vec<f64>,
    pub label_width: Vec<f64>,
    pub label_height: Vec<f64>,
    pub label_identifier: Vec<Option<String>>,
    pub label_properties: Vec<MapPropertyHolder>,
    pub label_text: Vec<String>,
    pub label_parent: Vec<Option<ELabelParent>>,

    // ── EdgeSection attributes (indexed by ESectionId) ──
    pub section_start_x: Vec<f64>,
    pub section_start_y: Vec<f64>,
    pub section_end_x: Vec<f64>,
    pub section_end_y: Vec<f64>,
    pub section_identifier: Vec<Option<String>>,
    pub section_properties: Vec<MapPropertyHolder>,
    pub section_parent_edge: Vec<EEdgeId>,
    pub section_bend_points: Vec<Vec<EBendId>>,
    pub section_outgoing_shape: Vec<Option<EConnectableId>>,
    pub section_incoming_shape: Vec<Option<EConnectableId>>,

    // ── BendPoint attributes (indexed by EBendId) ──
    pub bend_x: Vec<f64>,
    pub bend_y: Vec<f64>,
}

impl ElkGraphArena {
    pub fn new() -> Self {
        ElkGraphArena {
            node_x: Vec::new(), node_y: Vec::new(),
            node_width: Vec::new(), node_height: Vec::new(),
            node_identifier: Vec::new(), node_properties: Vec::new(),
            node_parent: Vec::new(),
            node_children: Vec::new(), node_ports: Vec::new(),
            node_contained_edges: Vec::new(), node_labels: Vec::new(),
            node_incoming_edges: Vec::new(), node_outgoing_edges: Vec::new(),

            port_x: Vec::new(), port_y: Vec::new(),
            port_width: Vec::new(), port_height: Vec::new(),
            port_identifier: Vec::new(), port_properties: Vec::new(),
            port_owner: Vec::new(),
            port_labels: Vec::new(),
            port_incoming_edges: Vec::new(), port_outgoing_edges: Vec::new(),

            edge_identifier: Vec::new(), edge_properties: Vec::new(),
            edge_containing_node: Vec::new(),
            edge_sources: Vec::new(), edge_targets: Vec::new(),
            edge_sections: Vec::new(), edge_labels: Vec::new(),

            label_x: Vec::new(), label_y: Vec::new(),
            label_width: Vec::new(), label_height: Vec::new(),
            label_identifier: Vec::new(), label_properties: Vec::new(),
            label_text: Vec::new(),
            label_parent: Vec::new(),

            section_start_x: Vec::new(), section_start_y: Vec::new(),
            section_end_x: Vec::new(), section_end_y: Vec::new(),
            section_identifier: Vec::new(), section_properties: Vec::new(),
            section_parent_edge: Vec::new(), section_bend_points: Vec::new(),
            section_outgoing_shape: Vec::new(), section_incoming_shape: Vec::new(),

            bend_x: Vec::new(), bend_y: Vec::new(),
        }
    }

    // ── Add methods (return new ID) ──

    pub fn add_node(&mut self, parent: Option<ENodeId>) -> ENodeId {
        let id = ENodeId(self.node_x.len() as u32);
        self.node_x.push(0.0); self.node_y.push(0.0);
        self.node_width.push(0.0); self.node_height.push(0.0);
        self.node_identifier.push(None);
        self.node_properties.push(MapPropertyHolder::new());
        self.node_parent.push(parent);
        self.node_children.push(Vec::new());
        self.node_ports.push(Vec::new());
        self.node_contained_edges.push(Vec::new());
        self.node_labels.push(Vec::new());
        self.node_incoming_edges.push(Vec::new());
        self.node_outgoing_edges.push(Vec::new());
        if let Some(pid) = parent {
            self.node_children[pid.idx()].push(id);
        }
        id
    }

    pub fn add_port(&mut self, owner: ENodeId) -> EPortId {
        let id = EPortId(self.port_x.len() as u32);
        self.port_x.push(0.0); self.port_y.push(0.0);
        self.port_width.push(0.0); self.port_height.push(0.0);
        self.port_identifier.push(None);
        self.port_properties.push(MapPropertyHolder::new());
        self.port_owner.push(owner);
        self.port_labels.push(Vec::new());
        self.port_incoming_edges.push(Vec::new());
        self.port_outgoing_edges.push(Vec::new());
        self.node_ports[owner.idx()].push(id);
        id
    }

    pub fn add_edge(&mut self, containing_node: Option<ENodeId>) -> EEdgeId {
        let id = EEdgeId(self.edge_identifier.len() as u32);
        self.edge_identifier.push(None);
        self.edge_properties.push(MapPropertyHolder::new());
        self.edge_containing_node.push(containing_node);
        self.edge_sources.push(Vec::new());
        self.edge_targets.push(Vec::new());
        self.edge_sections.push(Vec::new());
        self.edge_labels.push(Vec::new());
        if let Some(nid) = containing_node {
            self.node_contained_edges[nid.idx()].push(id);
        }
        id
    }

    pub fn add_edge_source(&mut self, edge: EEdgeId, source: EConnectableId) {
        self.edge_sources[edge.idx()].push(source);
        match source {
            EConnectableId::Node(nid) => self.node_outgoing_edges[nid.idx()].push(edge),
            EConnectableId::Port(pid) => self.port_outgoing_edges[pid.idx()].push(edge),
        }
    }

    pub fn add_edge_target(&mut self, edge: EEdgeId, target: EConnectableId) {
        self.edge_targets[edge.idx()].push(target);
        match target {
            EConnectableId::Node(nid) => self.node_incoming_edges[nid.idx()].push(edge),
            EConnectableId::Port(pid) => self.port_incoming_edges[pid.idx()].push(edge),
        }
    }

    pub fn add_label(&mut self, text: String) -> ELabelId {
        let id = ELabelId(self.label_x.len() as u32);
        self.label_x.push(0.0); self.label_y.push(0.0);
        self.label_width.push(0.0); self.label_height.push(0.0);
        self.label_identifier.push(None);
        self.label_properties.push(MapPropertyHolder::new());
        self.label_text.push(text);
        self.label_parent.push(None);
        id
    }

    pub fn add_node_label(&mut self, node: ENodeId, label: ELabelId) {
        self.node_labels[node.idx()].push(label);
        self.label_parent[label.idx()] = Some(ELabelParent::Node(node));
    }

    pub fn add_port_label(&mut self, port: EPortId, label: ELabelId) {
        self.port_labels[port.idx()].push(label);
        self.label_parent[label.idx()] = Some(ELabelParent::Port(port));
    }

    pub fn add_edge_label(&mut self, edge: EEdgeId, label: ELabelId) {
        self.edge_labels[edge.idx()].push(label);
        self.label_parent[label.idx()] = Some(ELabelParent::Edge(edge));
    }

    pub fn add_section(&mut self, parent_edge: EEdgeId) -> ESectionId {
        let id = ESectionId(self.section_start_x.len() as u32);
        self.section_start_x.push(0.0); self.section_start_y.push(0.0);
        self.section_end_x.push(0.0); self.section_end_y.push(0.0);
        self.section_identifier.push(None);
        self.section_properties.push(MapPropertyHolder::new());
        self.section_parent_edge.push(parent_edge);
        self.section_bend_points.push(Vec::new());
        self.section_outgoing_shape.push(None);
        self.section_incoming_shape.push(None);
        self.edge_sections[parent_edge.idx()].push(id);
        id
    }

    pub fn add_bend_point(&mut self, section: ESectionId, x: f64, y: f64) -> EBendId {
        let id = EBendId(self.bend_x.len() as u32);
        self.bend_x.push(x);
        self.bend_y.push(y);
        self.section_bend_points[section.idx()].push(id);
        id
    }

    // ── Count methods ──

    #[inline] pub fn node_count(&self) -> usize { self.node_x.len() }
    #[inline] pub fn port_count(&self) -> usize { self.port_x.len() }
    #[inline] pub fn edge_count(&self) -> usize { self.edge_identifier.len() }
    #[inline] pub fn label_count(&self) -> usize { self.label_x.len() }
    #[inline] pub fn section_count(&self) -> usize { self.section_start_x.len() }
    #[inline] pub fn bend_count(&self) -> usize { self.bend_x.len() }
}
