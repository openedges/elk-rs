use std::cell::RefCell;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::Direction;

use super::{CGroupRef, CNodeRef};

pub type CGraphRef = Rc<RefCell<CGraph>>;

pub struct CGraph {
    pub c_nodes: Vec<CNodeRef>,
    pub c_groups: Vec<CGroupRef>,
    supported_directions: Vec<Direction>,
    pub predefined_horizontal_constraints: Vec<(CNodeRef, CNodeRef)>,
    pub predefined_vertical_constraints: Vec<(CNodeRef, CNodeRef)>,
}

impl CGraph {
    pub fn new(supported_directions: Vec<Direction>) -> CGraphRef {
        Rc::new(RefCell::new(CGraph {
            c_nodes: Vec::new(),
            c_groups: Vec::new(),
            supported_directions,
            predefined_horizontal_constraints: Vec::new(),
            predefined_vertical_constraints: Vec::new(),
        }))
    }

    pub fn all_directions() -> CGraphRef {
        Self::new(vec![
            Direction::Left,
            Direction::Right,
            Direction::Up,
            Direction::Down,
        ])
    }

    pub fn supports(&self, direction: Direction) -> bool {
        self.supported_directions.contains(&direction)
    }
}
