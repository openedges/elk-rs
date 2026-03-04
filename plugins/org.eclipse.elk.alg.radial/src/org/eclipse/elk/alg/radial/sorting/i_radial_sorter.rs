use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub trait IRadialSorter: Send {
    fn sort(&mut self, nodes: &mut Vec<ElkNodeRef>);
    fn initialize(&mut self, root: &ElkNodeRef);

    /// Sort successors of a parent node directly, without full-tree initialize.
    /// PolarCoordinateSorter overrides this for O(k log k) direct polar sort
    /// instead of O(N) full-tree traversal in initialize().
    fn sort_for_parent(
        &mut self,
        nodes: &mut Vec<ElkNodeRef>,
        _parent: &ElkNodeRef,
        root: &ElkNodeRef,
        _is_root_level: bool,
    ) {
        self.initialize(root);
        self.sort(nodes);
    }
}
