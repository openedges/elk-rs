use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;

#[derive(Clone, Debug)]
pub struct TEdge {
    pub u: KVector,
    pub v: KVector,
}

impl TEdge {
    pub fn new(u: KVector, v: KVector) -> Self {
        TEdge { u, v }
    }

    fn ordered_endpoints(&self) -> (&KVector, &KVector) {
        if cmp_kvector(&self.u, &self.v) == Ordering::Greater {
            (&self.v, &self.u)
        } else {
            (&self.u, &self.v)
        }
    }
}

impl PartialEq for TEdge {
    fn eq(&self, other: &Self) -> bool {
        (self.u == other.u && self.v == other.v) || (self.u == other.v && self.v == other.u)
    }
}

impl Eq for TEdge {}

impl Hash for TEdge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (first, second) = self.ordered_endpoints();
        first.hash(state);
        second.hash(state);
    }
}

fn cmp_kvector(a: &KVector, b: &KVector) -> Ordering {
    match a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal) {
        Ordering::Equal => a.y.partial_cmp(&b.y).unwrap_or(Ordering::Equal),
        ordering => ordering,
    }
}
