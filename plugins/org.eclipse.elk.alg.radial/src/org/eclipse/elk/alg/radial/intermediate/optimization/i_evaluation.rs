use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub trait IEvaluation {
    fn evaluate(&self, graph: &ElkNodeRef) -> f64;
}
