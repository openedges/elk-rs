use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub trait IRadialRotator {
    fn rotate(&mut self, graph: &ElkNodeRef);
}
