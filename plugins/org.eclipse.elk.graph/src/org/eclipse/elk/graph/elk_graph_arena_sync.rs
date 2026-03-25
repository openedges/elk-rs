use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkEdgeSectionRef, ElkGraphElementRef,
    ElkLabelRef, ElkNodeRef, ElkPortRef,
};
use crate::org::eclipse::elk::graph::elk_graph_arena::{
    EConnectableId, EEdgeId, ELabelId, ELabelParent, ENodeId, EPortId, ESectionId,
    ElkGraphArena,
};

/// Bidirectional bridge between the Rc<RefCell<T>> ElkGraph tree and ElkGraphArena.
pub struct ElkGraphArenaSync {
    arena: ElkGraphArena,
    node_rc_to_id: FxHashMap<usize, ENodeId>,
    port_rc_to_id: FxHashMap<usize, EPortId>,
    edge_rc_to_id: FxHashMap<usize, EEdgeId>,
    label_rc_to_id: FxHashMap<usize, ELabelId>,
    section_rc_to_id: FxHashMap<usize, ESectionId>,
    node_id_to_rc: Vec<ElkNodeRef>,
    port_id_to_rc: Vec<ElkPortRef>,
    edge_id_to_rc: Vec<ElkEdgeRef>,
    label_id_to_rc: Vec<ElkLabelRef>,
    section_id_to_rc: Vec<ElkEdgeSectionRef>,
}

impl ElkGraphArenaSync {
    /// Build arena from an ElkGraph root node tree.
    pub fn from_root(root: &ElkNodeRef) -> Self {
        let mut sync = ElkGraphArenaSync {
            arena: ElkGraphArena::new(),
            node_rc_to_id: FxHashMap::default(),
            port_rc_to_id: FxHashMap::default(),
            edge_rc_to_id: FxHashMap::default(),
            label_rc_to_id: FxHashMap::default(),
            section_rc_to_id: FxHashMap::default(),
            node_id_to_rc: Vec::new(),
            port_id_to_rc: Vec::new(),
            edge_id_to_rc: Vec::new(),
            label_id_to_rc: Vec::new(),
            section_id_to_rc: Vec::new(),
        };
        sync.import_node(root, None);
        sync.import_edges(root);
        sync
    }

    #[inline] pub fn arena(&self) -> &ElkGraphArena { &self.arena }
    #[inline] pub fn arena_mut(&mut self) -> &mut ElkGraphArena { &mut self.arena }
    #[inline] pub fn node_id(&self, n: &ElkNodeRef) -> Option<ENodeId> { self.node_rc_to_id.get(&rc_ptr(n)).copied() }
    #[inline] pub fn port_id(&self, p: &ElkPortRef) -> Option<EPortId> { self.port_rc_to_id.get(&rc_ptr(p)).copied() }
    #[inline] pub fn edge_id(&self, e: &ElkEdgeRef) -> Option<EEdgeId> { self.edge_rc_to_id.get(&rc_ptr(e)).copied() }
    #[inline] pub fn label_id(&self, l: &ElkLabelRef) -> Option<ELabelId> { self.label_rc_to_id.get(&rc_ptr(l)).copied() }
    #[inline] pub fn section_id(&self, s: &ElkEdgeSectionRef) -> Option<ESectionId> { self.section_rc_to_id.get(&rc_ptr(s)).copied() }
    #[inline] pub fn node_ref(&self, id: ENodeId) -> &ElkNodeRef { &self.node_id_to_rc[id.idx()] }
    #[inline] pub fn port_ref(&self, id: EPortId) -> &ElkPortRef { &self.port_id_to_rc[id.idx()] }
    #[inline] pub fn edge_ref(&self, id: EEdgeId) -> &ElkEdgeRef { &self.edge_id_to_rc[id.idx()] }
    #[inline] pub fn label_ref(&self, id: ELabelId) -> &ElkLabelRef { &self.label_id_to_rc[id.idx()] }
    #[inline] pub fn section_ref(&self, id: ESectionId) -> &ElkEdgeSectionRef { &self.section_id_to_rc[id.idx()] }

    /// Resolve a connectable ID to the owning node ID.
    pub fn connectable_node_id(&self, cid: EConnectableId) -> ENodeId {
        match cid {
            EConnectableId::Node(nid) => nid,
            EConnectableId::Port(pid) => self.arena.port_owner[pid.idx()],
        }
    }

    /// Resolve a label's parent as ElkGraphElementRef via arena.
    pub fn label_parent_ref(&self, lid: ELabelId) -> Option<ElkGraphElementRef> {
        self.arena.label_parent[lid.idx()].map(|lp| match lp {
            ELabelParent::Node(nid) => ElkGraphElementRef::Node(self.node_ref(nid).clone()),
            ELabelParent::Port(pid) => ElkGraphElementRef::Port(self.port_ref(pid).clone()),
            ELabelParent::Edge(eid) => ElkGraphElementRef::Edge(self.edge_ref(eid).clone()),
        })
    }

    pub fn connectable_id(&self, shape: &ElkConnectableShapeRef) -> Option<EConnectableId> {
        match shape {
            ElkConnectableShapeRef::Node(n) => self.node_id(n).map(EConnectableId::Node),
            ElkConnectableShapeRef::Port(p) => self.port_id(p).map(EConnectableId::Port),
        }
    }

    /// Write node/port positions and sizes from arena back to Rc tree.
    pub fn sync_positions_to_tree(&self) {
        let a = &self.arena;
        for (i, node_ref) in self.node_id_to_rc.iter().enumerate() {
            let mut n = node_ref.borrow_mut();
            let s = n.connectable().shape();
            s.set_x(a.node_x[i]);
            s.set_y(a.node_y[i]);
            s.set_width(a.node_width[i]);
            s.set_height(a.node_height[i]);
        }
        for (i, port_ref) in self.port_id_to_rc.iter().enumerate() {
            let mut p = port_ref.borrow_mut();
            let s = p.connectable().shape();
            s.set_x(a.port_x[i]);
            s.set_y(a.port_y[i]);
            s.set_width(a.port_width[i]);
            s.set_height(a.port_height[i]);
        }
    }

    /// Write label positions from arena back to Rc tree.
    pub fn sync_labels_to_tree(&self) {
        let a = &self.arena;
        for (i, label_ref) in self.label_id_to_rc.iter().enumerate() {
            let mut l = label_ref.borrow_mut();
            l.shape().set_x(a.label_x[i]);
            l.shape().set_y(a.label_y[i]);
            l.shape().set_width(a.label_width[i]);
            l.shape().set_height(a.label_height[i]);
        }
    }

    // ── Internal import ──

    fn import_node(&mut self, node_ref: &ElkNodeRef, parent: Option<ENodeId>) {
        let nid = self.arena.add_node(parent);
        self.node_rc_to_id.insert(rc_ptr(node_ref), nid);
        self.node_id_to_rc.push(node_ref.clone());

        {
            let mut n = node_ref.borrow_mut();
            let s = n.connectable().shape();
            self.arena.node_x[nid.idx()] = s.x();
            self.arena.node_y[nid.idx()] = s.y();
            self.arena.node_width[nid.idx()] = s.width();
            self.arena.node_height[nid.idx()] = s.height();
            let e = s.graph_element();
            self.arena.node_identifier[nid.idx()] = e.identifier().map(|s| s.to_string());
            self.arena.node_properties[nid.idx()] = e.properties().clone();
        }

        // Labels
        let labels: Vec<ElkLabelRef> = {
            let mut n = node_ref.borrow_mut();
            n.connectable().shape().graph_element().labels().iter().cloned().collect()
        };
        for label_ref in &labels {
            self.import_label(label_ref);
            if let Some(lid) = self.label_id(label_ref) {
                self.arena.add_node_label(nid, lid);
            }
        }

        // Ports
        let ports: Vec<ElkPortRef> = node_ref.borrow_mut().ports().iter().cloned().collect();
        for port_ref in &ports {
            self.import_port(port_ref, nid);
        }

        // Children (recursive)
        let children: Vec<ElkNodeRef> = node_ref.borrow_mut().children().iter().cloned().collect();
        for child_ref in &children {
            self.import_node(child_ref, Some(nid));
        }
    }

    fn import_port(&mut self, port_ref: &ElkPortRef, owner: ENodeId) {
        let pid = self.arena.add_port(owner);
        self.port_rc_to_id.insert(rc_ptr(port_ref), pid);
        self.port_id_to_rc.push(port_ref.clone());

        {
            let mut p = port_ref.borrow_mut();
            let s = p.connectable().shape();
            self.arena.port_x[pid.idx()] = s.x();
            self.arena.port_y[pid.idx()] = s.y();
            self.arena.port_width[pid.idx()] = s.width();
            self.arena.port_height[pid.idx()] = s.height();
            let e = s.graph_element();
            self.arena.port_identifier[pid.idx()] = e.identifier().map(|s| s.to_string());
            self.arena.port_properties[pid.idx()] = e.properties().clone();
        }

        let labels: Vec<ElkLabelRef> = {
            let mut p = port_ref.borrow_mut();
            p.connectable().shape().graph_element().labels().iter().cloned().collect()
        };
        for label_ref in &labels {
            self.import_label(label_ref);
            if let Some(lid) = self.label_id(label_ref) {
                self.arena.add_port_label(pid, lid);
            }
        }
    }

    fn import_label(&mut self, label_ref: &ElkLabelRef) {
        let text = {
            let l = label_ref.borrow();
            l.text().to_string()
        };
        let lid = self.arena.add_label(text);
        self.label_rc_to_id.insert(rc_ptr(label_ref), lid);
        self.label_id_to_rc.push(label_ref.clone());

        {
            let mut l = label_ref.borrow_mut();
            let s = l.shape();
            self.arena.label_x[lid.idx()] = s.x();
            self.arena.label_y[lid.idx()] = s.y();
            self.arena.label_width[lid.idx()] = s.width();
            self.arena.label_height[lid.idx()] = s.height();
            let e = s.graph_element();
            self.arena.label_identifier[lid.idx()] = e.identifier().map(|s| s.to_string());
            self.arena.label_properties[lid.idx()] = e.properties().clone();
        }
    }

    fn import_edges(&mut self, root: &ElkNodeRef) {
        self.import_edges_of_node(root);
        let children: Vec<ElkNodeRef> = root.borrow_mut().children().iter().cloned().collect();
        for child in &children {
            self.import_edges_recursive(child);
        }
    }

    fn import_edges_recursive(&mut self, node: &ElkNodeRef) {
        self.import_edges_of_node(node);
        let children: Vec<ElkNodeRef> = node.borrow_mut().children().iter().cloned().collect();
        for child in &children {
            self.import_edges_recursive(child);
        }
    }

    fn import_edges_of_node(&mut self, node: &ElkNodeRef) {
        let containing_nid = self.node_id(node);
        let edges: Vec<ElkEdgeRef> = node.borrow_mut().contained_edges().iter().cloned().collect();

        for edge_ref in &edges {
            if self.edge_rc_to_id.contains_key(&rc_ptr(edge_ref)) {
                continue;
            }

            let eid = self.arena.add_edge(containing_nid);
            self.edge_rc_to_id.insert(rc_ptr(edge_ref), eid);
            self.edge_id_to_rc.push(edge_ref.clone());

            {
                let mut e = edge_ref.borrow_mut();
                let el = e.element();
                self.arena.edge_identifier[eid.idx()] = el.identifier().map(|s| s.to_string());
                self.arena.edge_properties[eid.idx()] = el.properties().clone();
            }

            // Sources and targets
            {
                let mut e = edge_ref.borrow_mut();
                let sources: Vec<_> = e.sources().iter().cloned().collect();
                let targets: Vec<_> = e.targets().iter().cloned().collect();
                drop(e);
                for src in &sources {
                    if let Some(cid) = self.connectable_id(src) {
                        self.arena.add_edge_source(eid, cid);
                    }
                }
                for tgt in &targets {
                    if let Some(cid) = self.connectable_id(tgt) {
                        self.arena.add_edge_target(eid, cid);
                    }
                }
            }

            // Sections + bend points
            {
                let mut e = edge_ref.borrow_mut();
                let sections: Vec<_> = e.sections().iter().cloned().collect();
                drop(e);
                for section_ref in sections.iter() {
                    let sid = self.arena.add_section(eid);
                    self.section_rc_to_id.insert(rc_ptr(section_ref), sid);
                    self.section_id_to_rc.push(section_ref.clone());

                    let mut sec = section_ref.borrow_mut();
                    self.arena.section_start_x[sid.idx()] = sec.start_x();
                    self.arena.section_start_y[sid.idx()] = sec.start_y();
                    self.arena.section_end_x[sid.idx()] = sec.end_x();
                    self.arena.section_end_y[sid.idx()] = sec.end_y();
                    self.arena.section_identifier[sid.idx()] = sec.identifier().map(|s| s.to_string());
                    self.arena.section_properties[sid.idx()] = sec.properties().clone();

                    for bp in sec.bend_points() {
                        let bp_ref = bp.borrow();
                        self.arena.add_bend_point(sid, bp_ref.x(), bp_ref.y());
                    }
                }
            }

            // Edge labels
            {
                let mut e = edge_ref.borrow_mut();
                let labels: Vec<ElkLabelRef> = e.element().labels().iter().cloned().collect();
                drop(e);
                for label_ref in &labels {
                    self.import_label(label_ref);
                    if let Some(lid) = self.label_id(label_ref) {
                        self.arena.add_edge_label(eid, lid);
                    }
                }
            }
        }
    }
}

#[inline]
fn rc_ptr<T>(rc: &Rc<T>) -> usize {
    Rc::as_ptr(rc) as usize
}
