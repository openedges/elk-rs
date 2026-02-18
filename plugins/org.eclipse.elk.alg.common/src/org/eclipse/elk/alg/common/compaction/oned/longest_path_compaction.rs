use std::collections::VecDeque;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::Direction;

use super::{CGroupRef, ICompactionAlgorithm, OneDimensionalCompactor};

pub struct LongestPathCompaction;

impl ICompactionAlgorithm for LongestPathCompaction {
    fn compact(&self, compactor: &mut OneDimensionalCompactor) {
        let c_nodes = compactor.c_graph.borrow().c_nodes.clone();

        let mut min_start_pos = f64::INFINITY;
        for c_node in &c_nodes {
            let node_group = c_node.borrow().group();
            let Some(node_group) = node_group else {
                continue;
            };
            let reference = node_group.borrow().reference.clone();
            let Some(reference) = reference else {
                continue;
            };
            let reference_x = reference.borrow().hitbox.x;
            let offset_x = c_node.borrow().c_group_offset.x;
            min_start_pos = min_start_pos.min(reference_x + offset_x);
        }
        if !min_start_pos.is_finite() {
            min_start_pos = 0.0;
        }

        let c_groups = compactor.c_graph.borrow().c_groups.clone();
        let mut sinks: VecDeque<CGroupRef> = VecDeque::new();
        for group in &c_groups {
            let mut group_mut = group.borrow_mut();
            group_mut.start_pos = min_start_pos;
            if group_mut.out_degree == 0 {
                sinks.push_back(group.clone());
            }
        }

        while let Some(group) = sinks.pop_front() {
            let reference = group.borrow().reference.clone();
            let Some(reference) = reference else {
                continue;
            };

            let mut diff = reference.borrow().hitbox.x;

            let group_nodes = group.borrow().c_nodes.clone();
            for node in &group_nodes {
                let suggested_x = group.borrow().start_pos + node.borrow().c_group_offset.x;
                let current_x = node.borrow().hitbox.x;
                let can_move = !compactor.is_locked_group(&group, compactor.direction)
                    || current_x < suggested_x;
                let mut node_mut = node.borrow_mut();
                node_mut.start_pos = if can_move { suggested_x } else { current_x };
            }

            let reference_start_pos = reference.borrow().start_pos;
            diff -= reference_start_pos;

            {
                let mut group_mut = group.borrow_mut();
                group_mut.delta += diff;
                if matches!(compactor.direction, Direction::Right | Direction::Down) {
                    group_mut.delta_normalized += diff;
                } else {
                    group_mut.delta_normalized -= diff;
                }
            }

            for node in &group_nodes {
                let node_constraints = node.borrow().constraints.clone();
                let node_start_pos = node.borrow().start_pos;
                let node_width = node.borrow().hitbox.width;
                for inc_node in node_constraints {
                    let spacing = if compactor.direction.is_horizontal() {
                        compactor
                            .spacings_handler
                            .get_horizontal_spacing(node, &inc_node)
                    } else {
                        compactor
                            .spacings_handler
                            .get_vertical_spacing(node, &inc_node)
                    };

                    let inc_group = inc_node.borrow().group();
                    let Some(inc_group) = inc_group else {
                        continue;
                    };
                    let inc_offset_x = inc_node.borrow().c_group_offset.x;
                    {
                        let mut inc_group_mut = inc_group.borrow_mut();
                        inc_group_mut.start_pos = inc_group_mut
                            .start_pos
                            .max(node_start_pos + node_width + spacing - inc_offset_x);
                    }

                    if compactor.is_locked_node(&inc_node, compactor.direction) {
                        let locked_x = inc_node.borrow().hitbox.x - inc_offset_x;
                        let mut inc_group_mut = inc_group.borrow_mut();
                        inc_group_mut.start_pos = inc_group_mut.start_pos.max(locked_x);
                    }

                    let push_sink = {
                        let mut inc_group_mut = inc_group.borrow_mut();
                        inc_group_mut.out_degree -= 1;
                        inc_group_mut.out_degree == 0
                    };
                    if push_sink
                        && !sinks
                            .iter()
                            .any(|candidate| Rc::ptr_eq(candidate, &inc_group))
                    {
                        sinks.push_back(inc_group);
                    }
                }
            }
        }

        for c_node in &c_nodes {
            let start_pos = c_node.borrow().start_pos;
            c_node.borrow_mut().hitbox.x = start_pos;
        }
    }
}
