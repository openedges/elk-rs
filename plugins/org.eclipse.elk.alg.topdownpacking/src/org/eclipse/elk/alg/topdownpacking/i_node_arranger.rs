use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub trait INodeArranger {
    fn get_predicted_size(&self, graph: &ElkNodeRef) -> KVector;
}
