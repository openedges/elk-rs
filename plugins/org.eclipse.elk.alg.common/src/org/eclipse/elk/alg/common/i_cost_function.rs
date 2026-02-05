use super::TEdge;

pub trait ICostFunction: Send + Sync {
    fn cost(&self, edge: &TEdge) -> f64;
}

impl<F> ICostFunction for F
where
    F: Fn(&TEdge) -> f64 + Send + Sync,
{
    fn cost(&self, edge: &TEdge) -> f64 {
        self(edge)
    }
}
