use std::collections::HashMap;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::i_cost_function::ICostFunction;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::node::Node;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::t_edge::TEdge;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::utils::Utils;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMath, KVector};

use crate::org::eclipse::elk::alg::spore::graph::Graph;
use crate::org::eclipse::elk::alg::spore::options::SpanningTreeCostFunction;

pub struct GraphCostFunction {
    nodes: HashMap<KVector, Node>,
    cost_function: SpanningTreeCostFunction,
    preferred_root: Option<KVector>,
}

impl GraphCostFunction {
    pub fn new(graph: &Graph) -> Self {
        let mut nodes = HashMap::new();
        for node in &graph.vertices {
            nodes.insert(node.original_vertex, node.clone());
        }
        let preferred_root = graph.preferred_root().map(|node| node.vertex);
        GraphCostFunction {
            nodes,
            cost_function: graph.cost_function,
            preferred_root,
        }
    }
}

impl ICostFunction for GraphCostFunction {
    fn cost(&self, edge: &TEdge) -> f64 {
        match self.cost_function {
            SpanningTreeCostFunction::CenterDistance => edge.u.distance(&edge.v),
            SpanningTreeCostFunction::MinimumRootDistance => {
                let Some(root) = self.preferred_root else {
                    return edge.u.distance(&edge.v);
                };
                edge.u.distance(&root).min(edge.v.distance(&root))
            }
            SpanningTreeCostFunction::CircleUnderlap => {
                let (Some(n1), Some(n2)) = (self.nodes.get(&edge.u), self.nodes.get(&edge.v)) else {
                    return edge.u.distance(&edge.v);
                };
                edge.u.distance(&edge.v)
                    - edge.u.distance(&n1.rect.get_position())
                    - edge.v.distance(&n2.rect.get_position())
            }
            SpanningTreeCostFunction::RectangleUnderlap => {
                let (Some(n1), Some(n2)) = (self.nodes.get(&edge.u), self.nodes.get(&edge.v)) else {
                    return edge.u.distance(&edge.v);
                };
                n1.underlap(n2)
            }
            SpanningTreeCostFunction::InvertedOverlap => {
                let (Some(n1), Some(n2)) = (self.nodes.get(&edge.u), self.nodes.get(&edge.v)) else {
                    return edge.u.distance(&edge.v);
                };
                let dist = ElkMath::shortest_distance(&n1.rect, &n2.rect);
                if dist >= 0.0 {
                    return dist;
                }
                let s = n2.rect.get_center().distance(&n1.rect.get_center());
                -(Utils::overlap(&n1.rect, &n2.rect) - 1.0) * s
            }
        }
    }
}

pub struct InvertedCostFunction<C: ICostFunction> {
    inner: C,
}

impl<C: ICostFunction> InvertedCostFunction<C> {
    pub fn new(inner: C) -> Self {
        InvertedCostFunction { inner }
    }
}

impl<C: ICostFunction> ICostFunction for InvertedCostFunction<C> {
    fn cost(&self, edge: &TEdge) -> f64 {
        -self.inner.cost(edge)
    }
}
