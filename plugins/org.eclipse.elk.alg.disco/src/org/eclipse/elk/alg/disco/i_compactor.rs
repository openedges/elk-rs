use crate::org::eclipse::elk::alg::disco::graph::DCGraph;

pub trait ICompactor {
    fn compact(&mut self, graph: &mut DCGraph);
}
