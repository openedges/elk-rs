use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;

use super::t_graph_element::TGraphElement;

pub type TShapeRef = Arc<Mutex<TShape>>;

pub struct TShape {
    element: TGraphElement,
    position: KVector,
    size: KVector,
}

impl TShape {
    pub fn new(id: i32) -> Self {
        TShape {
            element: TGraphElement::new(id),
            position: KVector::new(),
            size: KVector::new(),
        }
    }

    pub fn element(&self) -> &TGraphElement {
        &self.element
    }

    pub fn element_mut(&mut self) -> &mut TGraphElement {
        &mut self.element
    }

    pub fn position(&mut self) -> &mut KVector {
        &mut self.position
    }

    pub fn position_ref(&self) -> &KVector {
        &self.position
    }

    pub fn size(&mut self) -> &mut KVector {
        &mut self.size
    }

    pub fn size_ref(&self) -> &KVector {
        &self.size
    }
}

impl Default for TShape {
    fn default() -> Self {
        Self::new(0)
    }
}
