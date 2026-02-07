use std::cell::RefCell;
use std::rc::Rc;

use super::{CGraphRef, CNodeRef};

pub type CGroupRef = Rc<RefCell<CGroup>>;

pub struct CGroup {
    pub id: i32,
    pub master: Option<CNodeRef>,
    pub c_nodes: Vec<CNodeRef>,
    pub start_pos: f64,
    pub incoming_constraints: Vec<CNodeRef>,
    pub out_degree: i32,
    pub out_degree_real: i32,
    pub reference: Option<CNodeRef>,
    pub delta: f64,
    pub delta_normalized: f64,
}

impl CGroup {
    pub fn create(graph: &CGraphRef, nodes: &[CNodeRef]) -> CGroupRef {
        let group = Rc::new(RefCell::new(CGroup {
            id: 0,
            master: None,
            c_nodes: Vec::new(),
            start_pos: f64::NEG_INFINITY,
            incoming_constraints: Vec::new(),
            out_degree: 0,
            out_degree_real: 0,
            reference: None,
            delta: 0.0,
            delta_normalized: 0.0,
        }));
        for node in nodes {
            Self::add_c_node(&group, node);
        }
        graph.borrow_mut().c_groups.push(group.clone());
        group
    }

    pub fn add_c_node(group: &CGroupRef, node: &CNodeRef) {
        if node.borrow().group().is_some() {
            panic!("CNode belongs to another CGroup");
        }
        {
            let mut group_mut = group.borrow_mut();
            if !group_mut.c_nodes.iter().any(|candidate| Rc::ptr_eq(candidate, node)) {
                group_mut.c_nodes.push(node.clone());
            }
            if group_mut.reference.is_none() {
                group_mut.reference = Some(node.clone());
            }
        }
        node.borrow_mut().c_group = Some(Rc::downgrade(group));
    }

    pub fn remove_c_node(group: &CGroupRef, node: &CNodeRef) -> bool {
        let removed = {
            let mut group_mut = group.borrow_mut();
            let original_len = group_mut.c_nodes.len();
            group_mut.c_nodes.retain(|candidate| !Rc::ptr_eq(candidate, node));
            original_len != group_mut.c_nodes.len()
        };
        if removed {
            node.borrow_mut().c_group = None;
        }
        removed
    }
}
