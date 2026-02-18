use std::cmp::Ordering;
use std::rc::Rc;

use crate::org::eclipse::elk::alg::common::compaction::Scanline;

use super::compare_fuzzy;
use super::{CNodeRef, IConstraintCalculationAlgorithm, OneDimensionalCompactor};

pub struct ScanlineConstraintCalculator;

#[derive(Clone)]
struct Timestamp {
    node: CNodeRef,
    low: bool,
}

impl ScanlineConstraintCalculator {
    fn sweep(compactor: &mut OneDimensionalCompactor, nodes: &[CNodeRef]) {
        let mut points = Vec::with_capacity(nodes.len() * 2);
        for node in nodes {
            points.push(Timestamp {
                node: node.clone(),
                low: true,
            });
            points.push(Timestamp {
                node: node.clone(),
                low: false,
            });
        }

        for (index, node) in nodes.iter().enumerate() {
            node.borrow_mut().id = index as i32;
        }

        let mut handler = ConstraintsScanlineHandler::new(nodes.len());
        Scanline::execute(points, timestamp_cmp, &mut |timestamp: &Timestamp| {
            handler.handle(timestamp, compactor);
        });
    }
}

impl IConstraintCalculationAlgorithm for ScanlineConstraintCalculator {
    fn calculate_constraints(&self, compactor: &mut OneDimensionalCompactor) {
        let nodes = compactor.c_graph.borrow().c_nodes.clone();
        Self::sweep(compactor, &nodes);
    }
}

fn timestamp_cmp(p1: &Timestamp, p2: &Timestamp) -> Ordering {
    let mut y1 = p1.node.borrow().hitbox.y;
    if !p1.low {
        y1 += p1.node.borrow().hitbox.height;
    }

    let mut y2 = p2.node.borrow().hitbox.y;
    if !p2.low {
        y2 += p2.node.borrow().hitbox.height;
    }

    let cmp = y1.partial_cmp(&y2).unwrap_or(Ordering::Equal);
    if cmp == Ordering::Equal {
        if !p1.low && p2.low {
            return Ordering::Less;
        }
        if !p2.low && p1.low {
            return Ordering::Greater;
        }
    }
    cmp
}

struct ConstraintsScanlineHandler {
    intervals: Vec<CNodeRef>,
    cand: Vec<Option<CNodeRef>>,
}

impl ConstraintsScanlineHandler {
    fn new(node_count: usize) -> Self {
        ConstraintsScanlineHandler {
            intervals: Vec::new(),
            cand: vec![None; node_count],
        }
    }

    fn handle(&mut self, timestamp: &Timestamp, compactor: &mut OneDimensionalCompactor) {
        if timestamp.low {
            self.insert(&timestamp.node);
        } else {
            self.delete(&timestamp.node, compactor);
        }
    }

    fn insert(&mut self, node: &CNodeRef) {
        let pos = self.insertion_pos(node);
        if self
            .intervals
            .get(pos)
            .is_some_and(|candidate| Rc::ptr_eq(candidate, node))
        {
            panic!("Invalid hitboxes for scanline constraint calculation");
        }
        self.intervals.insert(pos, node.clone());

        let node_id = node.borrow().id as usize;
        self.cand[node_id] = self.lower(node);

        if let Some(right) = self.higher(node) {
            let right_id = right.borrow().id as usize;
            self.cand[right_id] = Some(node.clone());
        }
    }

    fn delete(&mut self, node: &CNodeRef, compactor: &mut OneDimensionalCompactor) {
        let left = self.lower(node);
        let node_id = node.borrow().id as usize;
        if let Some(left) = left {
            if self.cand[node_id]
                .as_ref()
                .is_some_and(|candidate| Rc::ptr_eq(candidate, &left))
                && different_group(&left, node)
            {
                left.borrow_mut().constraints.push(node.clone());
            }
        }

        let right = self.higher(node);
        if let Some(right) = right {
            let right_id = right.borrow().id as usize;
            if self.cand[right_id]
                .as_ref()
                .is_some_and(|candidate| Rc::ptr_eq(candidate, node))
                && different_group(&right, node)
            {
                node.borrow_mut().constraints.push(right.clone());
            }
        }

        self.intervals
            .retain(|candidate| !Rc::ptr_eq(candidate, node));

        if let Some(floor) = self.floor(node) {
            if overlap(node, &floor) {
                let _ = compactor;
            }
        }
    }

    fn insertion_pos(&self, node: &CNodeRef) -> usize {
        let node_center = center_x(node);
        let mut pos = 0usize;
        while pos < self.intervals.len() && center_x(&self.intervals[pos]) < node_center {
            pos += 1;
        }
        pos
    }

    fn lower(&self, node: &CNodeRef) -> Option<CNodeRef> {
        let idx = self.index_of(node)?;
        if idx > 0 {
            Some(self.intervals[idx - 1].clone())
        } else {
            None
        }
    }

    fn higher(&self, node: &CNodeRef) -> Option<CNodeRef> {
        let idx = self.index_of(node)?;
        if idx + 1 < self.intervals.len() {
            Some(self.intervals[idx + 1].clone())
        } else {
            None
        }
    }

    fn floor(&self, node: &CNodeRef) -> Option<CNodeRef> {
        let idx = self.index_of(node)?;
        Some(self.intervals[idx].clone())
    }

    fn index_of(&self, node: &CNodeRef) -> Option<usize> {
        self.intervals
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, node))
    }
}

fn center_x(node: &CNodeRef) -> f64 {
    let node_ref = node.borrow();
    node_ref.hitbox.x + node_ref.hitbox.width / 2.0
}

fn different_group(a: &CNodeRef, b: &CNodeRef) -> bool {
    let group_a = a.borrow().group();
    let group_b = b.borrow().group();
    match (group_a, group_b) {
        (Some(group_a), Some(group_b)) => !Rc::ptr_eq(&group_a, &group_b),
        _ => true,
    }
}

fn overlap(n1: &CNodeRef, n2: &CNodeRef) -> bool {
    if Rc::ptr_eq(n1, n2) {
        return false;
    }
    let n1_ref = n1.borrow();
    let n2_ref = n2.borrow();
    compare_fuzzy::le(n1_ref.hitbox.x, n2_ref.hitbox.x + n2_ref.hitbox.width)
        && compare_fuzzy::le(n2_ref.hitbox.x, n1_ref.hitbox.x + n1_ref.hitbox.width)
}
