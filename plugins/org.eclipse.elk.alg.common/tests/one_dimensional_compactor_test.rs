use std::rc::Rc;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::compaction::oned::{
    CGraph, CGraphRef, CGroup, CNode, CNodeRef, ISpacingsHandler, OneDimensionalCompactor,
    QuadraticConstraintCalculation,
};
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::compaction::oned::scanline_constraint_calculator::ScanlineConstraintCalculator;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkRectangle;
use org_eclipse_elk_core::org::eclipse::elk::core::options::Direction;

const EPSILON: f64 = 0.0001;
const SPACING: f64 = 5.0;

struct TestSpacingHandler;

impl ISpacingsHandler for TestSpacingHandler {
    fn get_horizontal_spacing(&self, _c_node1: &CNodeRef, _c_node2: &CNodeRef) -> f64 {
        SPACING
    }

    fn get_vertical_spacing(&self, _c_node1: &CNodeRef, _c_node2: &CNodeRef) -> f64 {
        SPACING
    }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {expected}, got {actual}"
    );
}

fn node(graph: &CGraphRef, x: f64, y: f64, w: f64, h: f64) -> CNodeRef {
    CNode::create(graph, ElkRectangle::with_values(x, y, w, h))
}

fn compacter(graph: &CGraphRef) -> OneDimensionalCompactor {
    let mut compactor = OneDimensionalCompactor::new(graph.clone());
    compactor.set_constraint_algorithm(Box::new(QuadraticConstraintCalculation));
    compactor
}

#[test]
fn test_left_compaction() {
    let graph = CGraph::all_directions();
    let left = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let right = node(&graph, 30.0, 0.0, 20.0, 20.0);

    let mut compactor = compacter(&graph);
    compactor
        .change_direction(Direction::Left)
        .compact()
        .finish();

    assert_close(left.borrow().hitbox.x, 0.0);
    assert_close(right.borrow().hitbox.x, 20.0);
}

#[test]
fn test_left_compaction_equal_y_coordinate() {
    let graph = CGraph::all_directions();
    let top = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let bottom = node(&graph, 30.0, 20.0, 20.0, 20.0);

    let mut compactor = compacter(&graph);
    compactor.set_constraint_algorithm(Box::new(ScanlineConstraintCalculator));
    compactor
        .change_direction(Direction::Left)
        .compact()
        .finish();

    assert_close(top.borrow().hitbox.x, 0.0);
    assert_close(bottom.borrow().hitbox.x, 0.0);
}

#[test]
fn test_left_compaction_spacing_aware() {
    let graph = CGraph::all_directions();
    let left = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let right = node(&graph, 30.0, 20.0 + SPACING - 1.0, 20.0, 20.0);

    let mut compactor = compacter(&graph);
    compactor
        .set_spacings_handler(Box::new(TestSpacingHandler))
        .change_direction(Direction::Left)
        .compact()
        .finish();

    assert_close(left.borrow().hitbox.x, 0.0);
    assert_close(right.borrow().hitbox.x, 25.0);
}

#[test]
fn test_left_compaction_spacing_aware_2() {
    let graph = CGraph::all_directions();
    let left = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let right = node(&graph, 30.0, 20.0 + SPACING + 1.0, 20.0, 20.0);

    let mut compactor = compacter(&graph);
    compactor
        .set_spacings_handler(Box::new(TestSpacingHandler))
        .change_direction(Direction::Left)
        .compact()
        .finish();

    assert_close(left.borrow().hitbox.x, 0.0);
    assert_close(right.borrow().hitbox.x, 0.0);
}

#[test]
fn test_right_compaction() {
    let graph = CGraph::all_directions();
    let left = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let right = node(&graph, 30.0, 0.0, 20.0, 20.0);

    let mut compactor = compacter(&graph);
    compactor
        .change_direction(Direction::Right)
        .compact()
        .finish();

    assert_close(left.borrow().hitbox.x, 10.0);
    assert_close(right.borrow().hitbox.x, 30.0);
}

#[test]
fn test_up_compaction() {
    let graph = CGraph::all_directions();
    let upper = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let lower = node(&graph, 0.0, 30.0, 20.0, 20.0);

    let mut compactor = compacter(&graph);
    compactor.change_direction(Direction::Up).compact().finish();

    assert_close(upper.borrow().hitbox.y, 0.0);
    assert_close(lower.borrow().hitbox.y, 20.0);
}

#[test]
fn test_down_compaction() {
    let graph = CGraph::all_directions();
    let upper = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let lower = node(&graph, 0.0, 30.0, 20.0, 20.0);

    let mut compactor = compacter(&graph);
    compactor
        .change_direction(Direction::Down)
        .compact()
        .finish();

    assert_close(upper.borrow().hitbox.y, 10.0);
    assert_close(lower.borrow().hitbox.y, 30.0);
}

#[test]
fn test_left_group_compaction() {
    let graph = CGraph::all_directions();
    let left = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let upper_right = node(&graph, 40.0, 5.0, 20.0, 20.0);
    let lower_right = node(&graph, 30.0, 25.0, 20.0, 20.0);

    CGroup::create(&graph, &[upper_right.clone(), lower_right.clone()]);

    let mut compactor = compacter(&graph);
    compactor
        .change_direction(Direction::Left)
        .compact()
        .finish();

    assert_close(left.borrow().hitbox.x, 0.0);
    assert_close(upper_right.borrow().hitbox.x, 20.0);
    assert_close(lower_right.borrow().hitbox.x, 10.0);
}

#[test]
fn test_right_group_compaction() {
    let graph = CGraph::all_directions();
    let left = node(&graph, 0.0, 5.0, 20.0, 20.0);
    let upper_right = node(&graph, 40.0, 0.0, 20.0, 20.0);
    let lower_right = node(&graph, 10.0, 25.0, 20.0, 20.0);

    CGroup::create(&graph, &[left.clone(), lower_right.clone()]);

    let mut compactor = compacter(&graph);
    compactor
        .change_direction(Direction::Right)
        .compact()
        .finish();

    assert_close(left.borrow().hitbox.x, 20.0);
    assert_close(upper_right.borrow().hitbox.x, 40.0);
    assert_close(lower_right.borrow().hitbox.x, 30.0);
}

#[test]
fn test_up_group_compaction() {
    let graph = CGraph::all_directions();
    let upper_left = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let lower_left = node(&graph, 5.0, 40.0, 20.0, 20.0);
    let right = node(&graph, 25.0, 30.0, 20.0, 20.0);

    CGroup::create(&graph, &[lower_left.clone(), right.clone()]);

    let mut compactor = compacter(&graph);
    compactor.change_direction(Direction::Up).compact().finish();

    assert_close(upper_left.borrow().hitbox.y, 0.0);
    assert_close(lower_left.borrow().hitbox.y, 20.0);
    assert_close(right.borrow().hitbox.y, 10.0);
}

#[test]
fn test_down_group_compaction() {
    let graph = CGraph::all_directions();
    let upper_left = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let lower_left = node(&graph, 5.0, 40.0, 10.0, 20.0);
    let right = node(&graph, 25.0, 10.0, 20.0, 20.0);

    CGroup::create(&graph, &[upper_left.clone(), right.clone()]);

    let mut compactor = compacter(&graph);
    compactor
        .change_direction(Direction::Down)
        .compact()
        .finish();

    assert_close(upper_left.borrow().hitbox.y, 20.0);
    assert_close(lower_left.borrow().hitbox.y, 40.0);
    assert_close(right.borrow().hitbox.y, 30.0);
}

#[test]
fn test_no_spacing_applied_within_groups() {
    let graph = CGraph::all_directions();

    let one = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let two = node(&graph, 20.0, 10.0, 20.0, 20.0);
    let three = node(&graph, 40.0, 20.0, 20.0, 20.0);
    CGroup::create(&graph, &[one.clone(), two.clone(), three.clone()]);

    let four = node(&graph, 22.0, 80.0, 20.0, 20.0);
    let five = node(&graph, 42.0, 90.0, 20.0, 20.0);
    let six = node(&graph, 62.0, 100.0, 20.0, 20.0);
    CGroup::create(&graph, &[four.clone(), five.clone(), six.clone()]);

    let mut compactor = compacter(&graph);
    compactor
        .set_spacings_handler(Box::new(TestSpacingHandler))
        .change_direction(Direction::Left)
        .compact()
        .change_direction(Direction::Right)
        .compact()
        .change_direction(Direction::Up)
        .compact()
        .change_direction(Direction::Down)
        .compact()
        .finish();

    assert_close(one.borrow().hitbox.x, 0.0);
    assert_close(two.borrow().hitbox.x, 20.0);
    assert_close(three.borrow().hitbox.x, 40.0);
    assert_close(four.borrow().hitbox.x, 0.0);
    assert_close(five.borrow().hitbox.x, 20.0);
    assert_close(six.borrow().hitbox.x, 40.0);

    assert_close(one.borrow().hitbox.y, 0.0);
    assert_close(two.borrow().hitbox.y, 10.0);
    assert_close(three.borrow().hitbox.y, 20.0);
    assert_close(four.borrow().hitbox.y, 35.0);
    assert_close(five.borrow().hitbox.y, 45.0);
    assert_close(six.borrow().hitbox.y, 55.0);
}

#[test]
fn test_subsequent_directions_compaction() {
    let graph = CGraph::all_directions();
    let one = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let two = node(&graph, 25.0, 0.0, 20.0, 20.0);
    let three = node(&graph, 0.0, 25.0, 20.0, 20.0);
    let four = node(&graph, 25.0, 25.0, 20.0, 20.0);
    let directions = [
        Direction::Left,
        Direction::Right,
        Direction::Up,
        Direction::Down,
    ];

    for &d1 in &directions {
        for &d2 in &directions {
            for &d3 in &directions {
                for &d4 in &directions {
                    let mut compactor = compacter(&graph);
                    compactor
                        .set_spacings_handler(Box::new(TestSpacingHandler))
                        .change_direction(d1)
                        .compact()
                        .change_direction(d2)
                        .compact()
                        .change_direction(d3)
                        .compact()
                        .change_direction(d4)
                        .compact()
                        .finish();

                    let directions = format!("{d1:?} {d2:?} {d3:?} {d4:?}");
                    assert!(
                        (one.borrow().hitbox.x - 0.0).abs() <= EPSILON,
                        "{directions}: one.x"
                    );
                    assert!(
                        (one.borrow().hitbox.y - 0.0).abs() <= EPSILON,
                        "{directions}: one.y"
                    );
                    assert!(
                        (two.borrow().hitbox.x - 25.0).abs() <= EPSILON,
                        "{directions}: two.x"
                    );
                    assert!(
                        (two.borrow().hitbox.y - 0.0).abs() <= EPSILON,
                        "{directions}: two.y"
                    );
                    assert!(
                        (three.borrow().hitbox.x - 0.0).abs() <= EPSILON,
                        "{directions}: three.x"
                    );
                    assert!(
                        (three.borrow().hitbox.y - 25.0).abs() <= EPSILON,
                        "{directions}: three.y"
                    );
                    assert!(
                        (four.borrow().hitbox.x - 25.0).abs() <= EPSILON,
                        "{directions}: four.x"
                    );
                    assert!(
                        (four.borrow().hitbox.y - 25.0).abs() <= EPSILON,
                        "{directions}: four.y"
                    );
                }
            }
        }
    }
}

#[test]
fn test_horizontal_spacings() {
    let graph = CGraph::all_directions();
    let one = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let two = node(&graph, 50.0, 0.0, 20.0, 20.0);
    let three = node(&graph, 150.0, 0.0, 20.0, 20.0);

    let one_ref = one.clone();
    let two_ref = two.clone();
    let three_ref = three.clone();
    let spacing_handler = (
        move |c_node1: &CNodeRef, c_node2: &CNodeRef| -> f64 {
            if Rc::ptr_eq(c_node1, &three_ref) || Rc::ptr_eq(c_node2, &three_ref) {
                10.0
            } else if Rc::ptr_eq(c_node1, &two_ref) || Rc::ptr_eq(c_node2, &two_ref) {
                7.0
            } else if Rc::ptr_eq(c_node1, &one_ref) || Rc::ptr_eq(c_node2, &one_ref) {
                5.0
            } else {
                SPACING
            }
        },
        |_c_node1: &CNodeRef, _c_node2: &CNodeRef| -> f64 { SPACING },
    );

    let mut compactor = compacter(&graph);
    compactor
        .set_spacings_handler(Box::new(spacing_handler))
        .change_direction(Direction::Left)
        .compact()
        .finish();

    assert_close(one.borrow().hitbox.x, 0.0);
    assert_close(two.borrow().hitbox.x, 27.0);
    assert_close(three.borrow().hitbox.x, 57.0);

    let one_ref = one.clone();
    let two_ref = two.clone();
    let three_ref = three.clone();
    let spacing_handler = (
        move |c_node1: &CNodeRef, c_node2: &CNodeRef| -> f64 {
            if Rc::ptr_eq(c_node1, &three_ref) || Rc::ptr_eq(c_node2, &three_ref) {
                10.0
            } else if Rc::ptr_eq(c_node1, &two_ref) || Rc::ptr_eq(c_node2, &two_ref) {
                7.0
            } else if Rc::ptr_eq(c_node1, &one_ref) || Rc::ptr_eq(c_node2, &one_ref) {
                5.0
            } else {
                SPACING
            }
        },
        |_c_node1: &CNodeRef, _c_node2: &CNodeRef| -> f64 { SPACING },
    );

    let mut compactor = compacter(&graph);
    compactor
        .set_spacings_handler(Box::new(spacing_handler))
        .change_direction(Direction::Right)
        .compact()
        .finish();

    assert_close(one.borrow().hitbox.x, 0.0);
    assert_close(two.borrow().hitbox.x, 27.0);
    assert_close(three.borrow().hitbox.x, 57.0);

    let one_ref = one.clone();
    let two_ref = two.clone();
    let three_ref = three.clone();
    let spacing_handler = (
        move |c_node1: &CNodeRef, c_node2: &CNodeRef| -> f64 {
            if Rc::ptr_eq(c_node1, &three_ref) || Rc::ptr_eq(c_node2, &three_ref) {
                10.0
            } else if Rc::ptr_eq(c_node1, &two_ref) || Rc::ptr_eq(c_node2, &two_ref) {
                7.0
            } else if Rc::ptr_eq(c_node1, &one_ref) || Rc::ptr_eq(c_node2, &one_ref) {
                5.0
            } else {
                SPACING
            }
        },
        |_c_node1: &CNodeRef, _c_node2: &CNodeRef| -> f64 { SPACING },
    );

    let mut compactor = compacter(&graph);
    compactor
        .set_spacings_handler(Box::new(spacing_handler))
        .change_direction(Direction::Left)
        .compact()
        .finish();

    assert_close(one.borrow().hitbox.x, 0.0);
    assert_close(two.borrow().hitbox.x, 27.0);
    assert_close(three.borrow().hitbox.x, 57.0);
}

#[test]
fn test_vertical_spacings() {
    let graph = CGraph::all_directions();
    let one = node(&graph, 0.0, 0.0, 20.0, 20.0);
    let two = node(&graph, 0.0, 50.0, 20.0, 20.0);
    let three = node(&graph, 0.0, 150.0, 20.0, 20.0);

    let one_ref = one.clone();
    let two_ref = two.clone();
    let three_ref = three.clone();
    let spacing_handler = (
        |_c_node1: &CNodeRef, _c_node2: &CNodeRef| -> f64 { 0.0 },
        move |c_node1: &CNodeRef, c_node2: &CNodeRef| -> f64 {
            if Rc::ptr_eq(c_node1, &three_ref) || Rc::ptr_eq(c_node2, &three_ref) {
                10.0
            } else if Rc::ptr_eq(c_node1, &two_ref) || Rc::ptr_eq(c_node2, &two_ref) {
                7.0
            } else if Rc::ptr_eq(c_node1, &one_ref) || Rc::ptr_eq(c_node2, &one_ref) {
                5.0
            } else {
                0.0
            }
        },
    );

    let mut compactor = compacter(&graph);
    compactor
        .set_spacings_handler(Box::new(spacing_handler))
        .change_direction(Direction::Up)
        .compact()
        .finish();

    assert_close(one.borrow().hitbox.y, 0.0);
    assert_close(two.borrow().hitbox.y, 27.0);
    assert_close(three.borrow().hitbox.y, 57.0);

    let one_ref = one.clone();
    let two_ref = two.clone();
    let three_ref = three.clone();
    let spacing_handler = (
        |_c_node1: &CNodeRef, _c_node2: &CNodeRef| -> f64 { 0.0 },
        move |c_node1: &CNodeRef, c_node2: &CNodeRef| -> f64 {
            if Rc::ptr_eq(c_node1, &three_ref) || Rc::ptr_eq(c_node2, &three_ref) {
                10.0
            } else if Rc::ptr_eq(c_node1, &two_ref) || Rc::ptr_eq(c_node2, &two_ref) {
                7.0
            } else if Rc::ptr_eq(c_node1, &one_ref) || Rc::ptr_eq(c_node2, &one_ref) {
                5.0
            } else {
                0.0
            }
        },
    );

    let mut compactor = compacter(&graph);
    compactor
        .set_spacings_handler(Box::new(spacing_handler))
        .change_direction(Direction::Down)
        .compact()
        .finish();

    assert_close(one.borrow().hitbox.y, 0.0);
    assert_close(two.borrow().hitbox.y, 27.0);
    assert_close(three.borrow().hitbox.y, 57.0);

    let one_ref = one.clone();
    let two_ref = two.clone();
    let three_ref = three.clone();
    let spacing_handler = (
        |_c_node1: &CNodeRef, _c_node2: &CNodeRef| -> f64 { 0.0 },
        move |c_node1: &CNodeRef, c_node2: &CNodeRef| -> f64 {
            if Rc::ptr_eq(c_node1, &three_ref) || Rc::ptr_eq(c_node2, &three_ref) {
                10.0
            } else if Rc::ptr_eq(c_node1, &two_ref) || Rc::ptr_eq(c_node2, &two_ref) {
                7.0
            } else if Rc::ptr_eq(c_node1, &one_ref) || Rc::ptr_eq(c_node2, &one_ref) {
                5.0
            } else {
                0.0
            }
        },
    );

    let mut compactor = compacter(&graph);
    compactor
        .set_spacings_handler(Box::new(spacing_handler))
        .change_direction(Direction::Up)
        .compact()
        .finish();

    assert_close(one.borrow().hitbox.y, 0.0);
    assert_close(two.borrow().hitbox.y, 27.0);
    assert_close(three.borrow().hitbox.y, 57.0);
}

#[test]
fn test_vertical_spacing_during_horizontal_compaction() {
    let graph = CGraph::all_directions();
    let one = node(&graph, 150.0, 11.0, 20.0, 20.0);
    let two = node(&graph, 0.0, 40.0, 20.0, 20.0);
    let three = node(&graph, 150.0, 76.0, 20.0, 20.0);

    let one_ref = one.clone();
    let two_ref = two.clone();
    let three_ref = three.clone();
    let spacing_handler = (
        |_c_node1: &CNodeRef, _c_node2: &CNodeRef| -> f64 { 0.0 },
        move |c_node1: &CNodeRef, c_node2: &CNodeRef| -> f64 {
            if Rc::ptr_eq(c_node1, &three_ref) || Rc::ptr_eq(c_node2, &three_ref) {
                15.0
            } else if Rc::ptr_eq(c_node1, &one_ref) || Rc::ptr_eq(c_node2, &one_ref) {
                10.0
            } else if Rc::ptr_eq(c_node1, &two_ref) || Rc::ptr_eq(c_node2, &two_ref) {
                5.0
            } else {
                0.0
            }
        },
    );

    let mut compactor = compacter(&graph);
    compactor
        .set_spacings_handler(Box::new(spacing_handler))
        .change_direction(Direction::Left)
        .compact()
        .finish();

    assert_close(one.borrow().hitbox.x, 20.0);
    assert_close(two.borrow().hitbox.x, 0.0);
    assert_close(three.borrow().hitbox.x, 0.0);
}

#[test]
fn test_horizontal_spacing_during_vertical_compaction() {
    let graph = CGraph::all_directions();
    let one = node(&graph, 16.0, 150.0, 20.0, 20.0);
    let two = node(&graph, 40.0, 0.0, 20.0, 20.0);
    let three = node(&graph, 76.0, 150.0, 20.0, 20.0);

    let one_ref = one.clone();
    let two_ref = two.clone();
    let three_ref = three.clone();
    let spacing_handler = (
        move |c_node1: &CNodeRef, c_node2: &CNodeRef| -> f64 {
            if Rc::ptr_eq(c_node1, &three_ref) || Rc::ptr_eq(c_node2, &three_ref) {
                15.0
            } else if Rc::ptr_eq(c_node1, &two_ref) || Rc::ptr_eq(c_node2, &two_ref) {
                10.0
            } else if Rc::ptr_eq(c_node1, &one_ref) || Rc::ptr_eq(c_node2, &one_ref) {
                5.0
            } else {
                0.0
            }
        },
        |_c_node1: &CNodeRef, _c_node2: &CNodeRef| -> f64 { 0.0 },
    );

    let mut compactor = compacter(&graph);
    compactor
        .set_spacings_handler(Box::new(spacing_handler))
        .change_direction(Direction::Up)
        .compact()
        .finish();

    assert_close(one.borrow().hitbox.y, 20.0);
    assert_close(two.borrow().hitbox.y, 0.0);
    assert_close(three.borrow().hitbox.y, 0.0);
}
