use std::rc::Rc;

use super::compare_fuzzy;
use super::{IConstraintCalculationAlgorithm, OneDimensionalCompactor};

pub struct QuadraticConstraintCalculation;

impl IConstraintCalculationAlgorithm for QuadraticConstraintCalculation {
    fn calculate_constraints(&self, compactor: &mut OneDimensionalCompactor) {
        let c_nodes = compactor.c_graph.borrow().c_nodes.clone();

        for c_node in &c_nodes {
            c_node.borrow_mut().constraints.clear();
        }

        for c_node1 in &c_nodes {
            for c_node2 in &c_nodes {
                if Rc::ptr_eq(c_node1, c_node2) {
                    continue;
                }

                let same_group = {
                    let group1 = c_node1.borrow().group();
                    let group2 = c_node2.borrow().group();
                    match (group1, group2) {
                        (Some(group1), Some(group2)) => Rc::ptr_eq(&group1, &group2),
                        _ => false,
                    }
                };
                if same_group {
                    continue;
                }

                let spacing = if compactor.direction.is_horizontal() {
                    compactor
                        .spacings_handler
                        .get_vertical_spacing(c_node1, c_node2)
                } else {
                    compactor
                        .spacings_handler
                        .get_horizontal_spacing(c_node1, c_node2)
                };

                let n1 = c_node1.borrow();
                let n2 = c_node2.borrow();
                let to_the_right = n2.hitbox.x > n1.hitbox.x
                    || (compare_fuzzy::eq(n1.hitbox.x, n2.hitbox.x)
                        && n1.hitbox.width < n2.hitbox.width);
                let overlap_low =
                    compare_fuzzy::gt(n2.hitbox.y + n2.hitbox.height + spacing, n1.hitbox.y);
                let overlap_high =
                    compare_fuzzy::lt(n2.hitbox.y, n1.hitbox.y + n1.hitbox.height + spacing);
                drop(n1);
                drop(n2);

                if to_the_right && overlap_low && overlap_high {
                    c_node1.borrow_mut().constraints.push(c_node2.clone());
                }
            }
        }
    }
}
