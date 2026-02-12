use std::collections::HashMap;
use std::rc::Rc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkEdgeRef, ElkGraphElementRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

use crate::org::eclipse::elk::alg::layered::graph::transform::elk_graph_importer::ElkGraphImporter;
use crate::org::eclipse::elk::alg::layered::graph::transform::elk_graph_layout_transferrer::
    ElkGraphLayoutTransferrer;
use crate::org::eclipse::elk::alg::layered::graph::transform::IGraphTransformer;
use crate::org::eclipse::elk::alg::layered::options::internal_properties::OriginId;
use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LGraphRef};

#[derive(Default)]
pub struct OriginStore {
    next_id: OriginId,
    elements: HashMap<OriginId, ElkGraphElementRef>,
    index: HashMap<usize, OriginId>,
    ledges: HashMap<OriginId, LEdgeRef>,
}

impl OriginStore {
    pub fn new() -> Self {
        OriginStore {
            next_id: 0,
            elements: HashMap::new(),
            index: HashMap::new(),
            ledges: HashMap::new(),
        }
    }

    pub fn store(&mut self, element: ElkGraphElementRef) -> OriginId {
        let key = element_key(&element);
        if let Some(existing) = self.index.get(&key) {
            return *existing;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.elements.insert(id, element);
        self.index.insert(key, id);
        id
    }

    pub fn get_id(&self, element: &ElkGraphElementRef) -> Option<OriginId> {
        let key = element_key(element);
        self.index.get(&key).cloned()
    }

    pub fn get(&self, id: OriginId) -> Option<ElkGraphElementRef> {
        self.elements.get(&id).cloned()
    }

    pub fn get_node(&self, id: OriginId) -> Option<ElkNodeRef> {
        match self.get(id) {
            Some(ElkGraphElementRef::Node(node)) => Some(node),
            _ => None,
        }
    }

    pub fn get_port(&self, id: OriginId) -> Option<ElkPortRef> {
        match self.get(id) {
            Some(ElkGraphElementRef::Port(port)) => Some(port),
            _ => None,
        }
    }

    pub fn get_edge(&self, id: OriginId) -> Option<ElkEdgeRef> {
        match self.get(id) {
            Some(ElkGraphElementRef::Edge(edge)) => Some(edge),
            _ => None,
        }
    }

    pub fn get_label(&self, id: OriginId) -> Option<ElkLabelRef> {
        match self.get(id) {
            Some(ElkGraphElementRef::Label(label)) => Some(label),
            _ => None,
        }
    }

    pub fn register_ledge(&mut self, id: OriginId, edge: LEdgeRef) {
        self.ledges.insert(id, edge);
    }

    pub fn get_ledge(&self, id: OriginId) -> Option<LEdgeRef> {
        self.ledges.get(&id).cloned()
    }
}

fn element_key(element: &ElkGraphElementRef) -> usize {
    match element {
        ElkGraphElementRef::Node(node) => Rc::as_ptr(node) as usize,
        ElkGraphElementRef::Port(port) => Rc::as_ptr(port) as usize,
        ElkGraphElementRef::Edge(edge) => Rc::as_ptr(edge) as usize,
        ElkGraphElementRef::Label(label) => Rc::as_ptr(label) as usize,
    }
}

pub struct ElkGraphTransformer {
    origin_store: OriginStore,
}

impl ElkGraphTransformer {
    pub fn new() -> Self {
        ElkGraphTransformer {
            origin_store: OriginStore::new(),
        }
    }
}

impl Default for ElkGraphTransformer {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphTransformer<ElkNodeRef> for ElkGraphTransformer {
    fn import_graph(&mut self, graph: &ElkNodeRef) -> LGraphRef {
        let mut importer = ElkGraphImporter::new(&mut self.origin_store);
        importer.import_graph(graph)
    }

    fn apply_layout(&mut self, layered_graph: &LGraphRef) {
        let transferrer = ElkGraphLayoutTransferrer::new(&self.origin_store);
        transferrer.apply_layout(layered_graph);
    }
}
