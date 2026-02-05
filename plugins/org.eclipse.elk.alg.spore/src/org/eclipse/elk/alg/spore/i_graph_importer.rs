use crate::org::eclipse::elk::alg::spore::graph::Graph;

pub trait IGraphImporter<T> {
    fn import_graph(&mut self, input_graph: &T) -> Graph;
    fn update_graph(&mut self, graph: &mut Graph);
    fn apply_positions(&mut self, graph: &Graph);
}
