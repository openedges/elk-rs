use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub trait IRadialCompactor {
    fn compact(&mut self, graph: &ElkNodeRef);
}
