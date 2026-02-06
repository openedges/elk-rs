use std::hash::{Hash, Hasher};

use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;

use crate::org::eclipse::elk::alg::common::spore::InternalProperties;
use crate::org::eclipse::elk::alg::common::t_edge::TEdge;

#[derive(Clone, Debug)]
pub struct TTriangle {
    pub a: KVector,
    pub b: KVector,
    pub c: KVector,
    pub t_edges: Vec<TEdge>,
    pub vertices: Vec<KVector>,
    circumcenter: KVector,
}

impl TTriangle {
    pub fn new(a: KVector, b: KVector, c: KVector) -> Self {
        let t_edges = vec![TEdge::new(a, b), TEdge::new(b, c), TEdge::new(c, a)];
        let vertices = vec![a, b, c];
        let circumcenter = calculate_circumcenter(&a, &b, &c);
        TTriangle {
            a,
            b,
            c,
            t_edges,
            vertices,
            circumcenter,
        }
    }

    pub fn get_circumcenter(&self) -> KVector {
        self.circumcenter
    }

    pub fn in_circumcircle(&self, v: &KVector) -> bool {
        fuzzy_compare(
            self.circumcenter.distance(v),
            self.circumcenter.distance(&self.a),
            InternalProperties::FUZZINESS,
        ) < 0
    }

    pub fn contains_edge(&self, edge: &TEdge) -> bool {
        self.t_edges.iter().any(|e| e == edge)
    }

    pub fn contains_vertex(&self, vertex: &KVector) -> bool {
        self.vertices.iter().any(|v| v == vertex)
    }
}

impl PartialEq for TTriangle {
    fn eq(&self, other: &Self) -> bool {
        self.contains_vertex(&other.a)
            && self.contains_vertex(&other.b)
            && self.contains_vertex(&other.c)
    }
}

impl Eq for TTriangle {}

impl Hash for TTriangle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.a.hash(state);
        self.b.hash(state);
        self.c.hash(state);
    }
}

fn calculate_circumcenter(a: &KVector, b: &KVector, c: &KVector) -> KVector {
    let mut ab = *b;
    ab.sub(a);
    let mut ac = *c;
    ac.sub(a);
    let mut bc = *c;
    bc.sub(b);
    let e = ab.x * (a.x + b.x) + ab.y * (a.y + b.y);
    let f = ac.x * (a.x + c.x) + ac.y * (a.y + c.y);
    let g = 2.0 * (ab.x * bc.y - ab.y * bc.x);

    let px = (ac.y * e - ab.y * f) / g;
    let py = (ab.x * f - ac.x * e) / g;
    KVector::with_values(px, py)
}

fn fuzzy_compare(a: f64, b: f64, eps: f64) -> i32 {
    if (a - b).abs() <= eps {
        0
    } else if a < b {
        -1
    } else {
        1
    }
}
