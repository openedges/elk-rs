use std::cell::RefCell;
use std::rc::{Rc, Weak};

use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkRectangle, KVector};

use super::{CGraphRef, CGroupRef};

pub type CNodeRef = Rc<RefCell<CNode>>;

pub struct CNode {
    pub id: i32,
    pub c_group: Option<Weak<RefCell<super::CGroup>>>,
    pub c_group_offset: KVector,
    pub hitbox_pre_compaction: Option<ElkRectangle>,
    pub hitbox: ElkRectangle,
    pub constraints: Vec<CNodeRef>,
    pub start_pos: f64,
}

impl CNode {
    pub fn create(graph: &CGraphRef, hitbox: ElkRectangle) -> CNodeRef {
        let node = Rc::new(RefCell::new(CNode {
            id: 0,
            c_group: None,
            c_group_offset: KVector::new(),
            hitbox_pre_compaction: None,
            hitbox,
            constraints: Vec::new(),
            start_pos: f64::NEG_INFINITY,
        }));
        graph.borrow_mut().c_nodes.push(node.clone());
        node
    }

    pub fn group(&self) -> Option<CGroupRef> {
        self.c_group.as_ref().and_then(Weak::upgrade)
    }
}
