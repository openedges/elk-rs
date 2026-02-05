use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub trait IRadialEdgeRouter {
    fn route_edges(&mut self, node: &ElkNodeRef);
}
