use super::node::Node;

pub trait IOverlapHandler: Send + Sync {
    fn handle(&self, n1: &Node, n2: &Node);
}

impl<F> IOverlapHandler for F
where
    F: Fn(&Node, &Node) + Send + Sync,
{
    fn handle(&self, n1: &Node, n2: &Node) {
        self(n1, n2)
    }
}
