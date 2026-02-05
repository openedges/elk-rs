use crate::org::eclipse::elk::alg::layered::graph::LGraphRef;

pub trait IGraphTransformer<T> {
    fn import_graph(&mut self, graph: &T) -> LGraphRef;
    fn apply_layout(&mut self, layered_graph: &LGraphRef);
}
