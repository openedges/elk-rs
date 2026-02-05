use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub trait IRadialSorter: Send {
    fn sort(&mut self, nodes: &mut Vec<ElkNodeRef>);
    fn initialize(&mut self, root: &ElkNodeRef);
}
