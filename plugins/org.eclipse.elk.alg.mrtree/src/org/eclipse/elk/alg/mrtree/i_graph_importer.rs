use crate::org::eclipse::elk::alg::mrtree::graph::TGraphRef;

pub trait IGraphImporter<T> {
    fn import_graph(&mut self, graph: &T) -> Option<TGraphRef>;
    fn apply_layout(&self, tgraph: &TGraphRef);
}
