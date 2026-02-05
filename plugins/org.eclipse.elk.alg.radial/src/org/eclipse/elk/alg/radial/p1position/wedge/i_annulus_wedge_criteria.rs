use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub trait IAnnulusWedgeCriteria {
    fn calculate_wedge_space(&self, node: &ElkNodeRef) -> f64;
}
