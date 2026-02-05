use crate::org::eclipse::elk::alg::layered::graph::LGraphRef;

#[derive(Default)]
pub struct ComponentsProcessor;

impl ComponentsProcessor {
    pub fn new() -> Self {
        ComponentsProcessor
    }

    pub fn split(&self, graph: &LGraphRef) -> Vec<LGraphRef> {
        vec![graph.clone()]
    }

    pub fn combine(&self, _components: &[LGraphRef], _target: &LGraphRef) {}
}
