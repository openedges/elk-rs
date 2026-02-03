use std::sync::{Arc, Mutex, Weak};

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;

use super::{index_of_arc, LGraphElement, LGraphRef, LGraphWeak, LNodeRef};

pub struct Layer {
    self_ref: Weak<Mutex<Layer>>,
    element: LGraphElement,
    owner: LGraphWeak,
    size: KVector,
    nodes: Vec<LNodeRef>,
}

impl Layer {
    pub fn new(graph: &LGraphRef) -> Arc<Mutex<Layer>> {
        Arc::new_cyclic(|weak| {
            Mutex::new(Layer {
                self_ref: weak.clone(),
                element: LGraphElement::new(),
                owner: Arc::downgrade(graph),
                size: KVector::new(),
                nodes: Vec::new(),
            })
        })
    }

    pub fn graph_element(&mut self) -> &mut LGraphElement {
        &mut self.element
    }

    pub fn graph(&self) -> Option<LGraphRef> {
        self.owner.upgrade()
    }

    pub fn size(&mut self) -> &mut KVector {
        &mut self.size
    }

    pub fn size_ref(&self) -> &KVector {
        &self.size
    }

    pub fn nodes(&self) -> &Vec<LNodeRef> {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut Vec<LNodeRef> {
        &mut self.nodes
    }

    pub fn index(&self) -> Option<usize> {
        let layer_ref = self.self_ref.upgrade()?;
        let graph = self.owner.upgrade()?;
        let graph_guard = graph.lock().ok()?;
        index_of_arc(graph_guard.layers(), &layer_ref)
    }

    pub fn to_string(&mut self) -> String {
        let index = self.index().map(|value| value.to_string()).unwrap_or_else(|| "-1".to_owned());
        let nodes = self
            .nodes
            .iter()
            .map(|node| node.lock().map(|mut n| n.to_string()).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(", ");
        format!("L_{}[{}]", index, nodes)
    }
}
