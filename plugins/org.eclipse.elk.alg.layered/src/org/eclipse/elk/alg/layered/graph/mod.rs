use std::sync::{Arc, Mutex, Weak};

pub mod l_edge;
pub mod l_graph;
pub mod l_graph_element;
pub mod l_graph_util;
pub mod l_label;
pub mod l_margin;
pub mod l_node;
pub mod l_padding;
pub mod l_port;
pub mod l_shape;
pub mod layer;
pub mod tarjan;
pub mod transform;

pub use l_edge::LEdge;
pub use l_graph::LGraph;
pub use l_graph_element::LGraphElement;
pub use l_graph_util::LGraphUtil;
pub use l_label::LLabel;
pub use l_margin::LMargin;
pub use l_node::{LNode, NodeType};
pub use l_padding::LPadding;
pub use l_port::LPort;
pub use l_shape::LShape;
pub use layer::Layer;
pub use tarjan::{NodeRefKey, Tarjan};

pub type LGraphRef = Arc<Mutex<LGraph>>;
pub type LGraphWeak = Weak<Mutex<LGraph>>;

pub type LayerRef = Arc<Mutex<Layer>>;
pub type LayerWeak = Weak<Mutex<Layer>>;

pub type LNodeRef = Arc<Mutex<LNode>>;
pub type LNodeWeak = Weak<Mutex<LNode>>;

pub type LPortRef = Arc<Mutex<LPort>>;
pub type LPortWeak = Weak<Mutex<LPort>>;

pub type LEdgeRef = Arc<Mutex<LEdge>>;
pub type LEdgeWeak = Weak<Mutex<LEdge>>;

pub type LLabelRef = Arc<Mutex<LLabel>>;
pub type LLabelWeak = Weak<Mutex<LLabel>>;

pub(crate) fn remove_arc<T>(items: &mut Vec<Arc<Mutex<T>>>, target: &Arc<Mutex<T>>) -> bool {
    if let Some(pos) = items.iter().position(|item| Arc::ptr_eq(item, target)) {
        items.remove(pos);
        true
    } else {
        false
    }
}

pub(crate) fn index_of_arc<T>(items: &[Arc<Mutex<T>>], target: &Arc<Mutex<T>>) -> Option<usize> {
    items.iter().position(|item| Arc::ptr_eq(item, target))
}
