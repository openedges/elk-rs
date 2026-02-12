use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::polyomino::structures::{
    IThreeValueGrid, TwoBitGrid,
};

const WIDTH: usize = 35;
const HEIGHT: usize = 3;

fn create_grid() -> TwoBitGrid {
    TwoBitGrid::new(WIDTH, HEIGHT)
}

#[test]
fn dimension_test() {
    let grid = create_grid();
    assert_eq!(grid.get_width(), WIDTH);
    assert_eq!(grid.get_height(), HEIGHT);

    let second = TwoBitGrid::new(64, 64);
    assert_eq!(second.get_width(), 64);
    assert_eq!(second.get_height(), 64);
}

#[test]
fn in_bounds_test() {
    let grid = create_grid();
    let neg_one = (-1_i32) as usize;
    assert!(grid.in_bounds(0, 0));
    assert!(!grid.in_bounds(neg_one, 0));
    assert!(!grid.in_bounds(0, neg_one));
    assert!(!grid.in_bounds(neg_one, neg_one));
    assert!(grid.in_bounds(34, 2));
    assert!(!grid.in_bounds(34, 3));
    assert!(!grid.in_bounds(35, 2));
    assert!(!grid.in_bounds(35, 3));
    assert!(grid.in_bounds(17, 1));
}

#[test]
fn cell_test() {
    let mut grid = create_grid();

    assert!(grid.is_empty(0, 0));
    assert!(grid.is_empty(34, 2));

    grid.set_blocked(0, 0);
    assert!(grid.is_blocked(0, 0));
    assert!(!grid.is_empty(0, 0));
    assert!(!grid.is_weakly_blocked(0, 0));

    grid.set_weakly_blocked(1, 0);
    assert!(grid.is_blocked(0, 0));
    assert!(!grid.is_empty(0, 0));
    assert!(!grid.is_weakly_blocked(0, 0));
    assert!(!grid.is_blocked(1, 0));
    assert!(!grid.is_empty(1, 0));
    assert!(grid.is_weakly_blocked(1, 0));

    grid.set_empty(0, 0);
    assert!(!grid.is_blocked(0, 0));
    assert!(grid.is_empty(0, 0));
    assert!(!grid.is_weakly_blocked(0, 0));
    assert!(!grid.is_blocked(1, 0));
    assert!(!grid.is_empty(1, 0));
    assert!(grid.is_weakly_blocked(1, 0));

    assert!(grid.is_empty(33, 2));

    grid.set_blocked(33, 2);
    assert!(!grid.is_blocked(0, 0));
    assert!(grid.is_empty(0, 0));
    assert!(!grid.is_weakly_blocked(0, 0));
    assert!(!grid.is_blocked(1, 0));
    assert!(!grid.is_empty(1, 0));
    assert!(grid.is_weakly_blocked(1, 0));
    assert!(grid.is_blocked(33, 2));
    assert!(!grid.is_empty(33, 2));
    assert!(!grid.is_weakly_blocked(33, 2));

    let mut second = TwoBitGrid::new(64, 64);
    for y in 0..second.get_height() {
        for x in 0..second.get_width() {
            second.set_blocked(x, y);
        }
    }
    for y in 0..second.get_height() {
        for x in 0..second.get_width() {
            assert!(second.is_blocked(x, y));
        }
    }

    for y in 0..second.get_height() {
        for x in 0..second.get_width() {
            second.set_weakly_blocked(x, y);
        }
    }
    for y in 0..second.get_height() {
        for x in 0..second.get_width() {
            assert!(second.is_weakly_blocked(x, y));
        }
    }

    for y in 0..second.get_height() {
        for x in 0..second.get_width() {
            second.set_empty(x, y);
        }
    }
    for y in 0..second.get_height() {
        for x in 0..second.get_width() {
            assert!(second.is_empty(x, y));
        }
    }
}
