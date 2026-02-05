use std::cmp::Ordering;

use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::alg::vertiflex::internal_properties::InternalProperties;

pub struct NodeComparator {
    invert: bool,
}

impl NodeComparator {
    pub fn new(invert: bool) -> Self {
        NodeComparator { invert }
    }

    pub fn compare(&self, a: &ElkNodeRef, b: &ElkNodeRef) -> Ordering {
        let ay = node_y(a);
        let by = node_y(b);
        let mut cmp = if self.invert {
            by.partial_cmp(&ay)
        } else {
            ay.partial_cmp(&by)
        }
        .unwrap_or(Ordering::Equal);

        if cmp == Ordering::Equal {
            let am = node_get_property(a, InternalProperties::NODE_MODEL_ORDER).unwrap_or(0);
            let bm = node_get_property(b, InternalProperties::NODE_MODEL_ORDER).unwrap_or(0);
            cmp = am.cmp(&bm);
        }
        cmp
    }
}

fn node_y(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().y()
}

fn node_get_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
) -> Option<T> {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    props.get_property(property)
}
