use crate::org::eclipse::elk::alg::force::graph::FGraph;

pub trait IGraphImporter<T> {
    fn import_graph(&mut self, graph: &T) -> Option<FGraph>;
    fn apply_layout(&self, graph: &FGraph);
}
