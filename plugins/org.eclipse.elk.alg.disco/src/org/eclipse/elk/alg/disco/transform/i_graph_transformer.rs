use crate::org::eclipse::elk::alg::disco::graph::DCGraph;

pub trait IGraphTransformer<G> {
    fn import_graph(&mut self, graph: &G) -> &mut DCGraph;
    fn apply_layout(&mut self);
}
