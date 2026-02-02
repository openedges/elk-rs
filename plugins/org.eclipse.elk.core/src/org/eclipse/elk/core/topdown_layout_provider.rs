use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::core::math::KVector;

pub trait ITopdownLayoutProvider: Send {
    fn get_predicted_graph_size(&self, graph: &ElkNodeRef) -> KVector;
}
