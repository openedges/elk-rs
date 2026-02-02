use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkEdgeRef, ElkGraphElementRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

pub trait CacheKey {
    fn cache_key(&self) -> usize;
}

impl CacheKey for ElkNodeRef {
    fn cache_key(&self) -> usize {
        std::rc::Rc::as_ptr(self) as usize
    }
}

impl CacheKey for ElkPortRef {
    fn cache_key(&self) -> usize {
        std::rc::Rc::as_ptr(self) as usize
    }
}

impl CacheKey for ElkEdgeRef {
    fn cache_key(&self) -> usize {
        std::rc::Rc::as_ptr(self) as usize
    }
}

impl CacheKey for ElkLabelRef {
    fn cache_key(&self) -> usize {
        std::rc::Rc::as_ptr(self) as usize
    }
}

impl CacheKey for ElkGraphElementRef {
    fn cache_key(&self) -> usize {
        match self {
            ElkGraphElementRef::Node(node) => node.cache_key(),
            ElkGraphElementRef::Edge(edge) => edge.cache_key(),
            ElkGraphElementRef::Port(port) => port.cache_key(),
            ElkGraphElementRef::Label(label) => label.cache_key(),
        }
    }
}
