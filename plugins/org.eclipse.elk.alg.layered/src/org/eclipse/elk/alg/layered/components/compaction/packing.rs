// Copyright (c) 2016 Kiel University and others.
// SPDX-License-Identifier: EPL-2.0 OR GPL-3.0-or-later
// Port of ComponentsToCGraphTransformer and OneDimensionalComponentsCompaction.

use std::rc::Rc;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::compaction::oned::{
    CGraph, CGraphRef, CGroup, CGroupRef, CNode, CNodeRef, ISpacingsHandler,
    OneDimensionalCompactor,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{Direction, PortSide};
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use rustc_hash::FxHashMap;

use super::Component;

const MAX_ITERATIONS: usize = 10;
const EPSILON: f64 = 0.0001;

struct Extension {
    group: CGroupRef,
    node: CNodeRef,
    placeholder: CNodeRef,
    placeholder_group: CGroupRef,
    side: PortSide,
}

struct Spacing {
    default: f64,
    individual: FxHashMap<usize, f64>,
}

impl ISpacingsHandler for Spacing {
    fn get_horizontal_spacing(&self, a: &CNodeRef, b: &CNodeRef) -> f64 {
        self.individual
            .get(&key(a))
            .copied()
            .unwrap_or(self.default)
            .min(
                self.individual
                    .get(&key(b))
                    .copied()
                    .unwrap_or(self.default),
            )
    }
    fn get_vertical_spacing(&self, a: &CNodeRef, b: &CNodeRef) -> f64 {
        self.get_horizontal_spacing(a, b)
    }
}

fn key(node: &CNodeRef) -> usize {
    Rc::as_ptr(node) as usize
}

fn locks(sides: &EnumSet<PortSide>) -> [bool; 4] {
    // Java's order is LEFT, RIGHT, UP, DOWN. Preserve its side-set mapping,
    // including the unspecified NORTH/EAST/SOUTH combination.
    let mask = sides.iter().fold(0, |bits, side| {
        bits | match side {
            PortSide::North => 1,
            PortSide::East => 2,
            PortSide::South => 4,
            PortSide::West => 8,
            PortSide::Undefined => 0,
        }
    });
    match mask {
        0 | 5 | 10 | 11 | 13 | 14 | 15 => [true; 4],
        1 => [true, true, true, false],
        2 => [false, true, true, true],
        4 => [true, true, false, true],
        8 => [true, false, true, true],
        3 => [false, true, true, false],
        6 => [false, true, false, true],
        12 => [true, false, false, true],
        9 => [true, false, true, false],
        _ => [false; 4],
    }
}

fn direction_index(direction: Direction) -> usize {
    match direction {
        Direction::Left => 0,
        Direction::Right => 1,
        Direction::Up => 2,
        Direction::Down => 3,
        Direction::Undefined => unreachable!("compaction has a direction"),
    }
}

pub(super) fn pack(components: &[Component], spacing: f64) -> (Vec<KVector>, KVector) {
    let graph = CGraph::all_directions();
    let mut references = Vec::new();
    let mut extensions = Vec::new();
    let mut node_locks = FxHashMap::default();
    let mut spacings = Spacing {
        default: spacing,
        individual: FxHashMap::default(),
    };

    for component in components {
        let hull_nodes: Vec<_> = component
            .hull
            .iter()
            .map(|rect| CNode::create(&graph, *rect))
            .collect();
        let reference = hull_nodes.first().expect("a component has a hull").clone();
        references.push((reference.clone(), reference.borrow().hitbox));
        let group = CGroup::create(&graph, &hull_nodes);
        let flags = locks(&component.sides);
        for node in &hull_nodes {
            node_locks.insert(key(node), flags);
        }
        for extension in &component.extensions {
            let node = CNode::create(&graph, extension.rectangle);
            CGroup::add_c_node(&group, &node);
            node_locks.insert(key(&node), flags);
            let placeholder = CNode::create(&graph, extension.placeholder);
            node_locks.insert(key(&placeholder), flags);
            let placeholder_group = CGroup::create(&graph, std::slice::from_ref(&placeholder));
            spacings.individual.insert(key(&placeholder), 1.0);
            extensions.push(Extension {
                group: group.clone(),
                node,
                placeholder,
                placeholder_group,
                side: extension.side,
            });
        }
    }

    // Placeholders are added only while compacting in their associated direction.
    for extension in &extensions {
        remove_node(&graph, &extension.placeholder);
        let group = extension.placeholder_group.clone();
        graph
            .borrow_mut()
            .c_groups
            .retain(|g| !Rc::ptr_eq(g, &group));
    }
    let mut top_left = KVector::with_values(f64::INFINITY, f64::INFINITY);
    for node in &graph.borrow().c_nodes {
        let rect = node.borrow().hitbox;
        top_left.x = top_left.x.min(rect.x);
        top_left.y = top_left.y.min(rect.y);
    }
    let mut compactor = OneDimensionalCompactor::new(graph.clone());
    compactor.set_spacings_handler(Box::new(spacings));
    for extension in &extensions {
        remove_node(&graph, &extension.node);
        CGroup::remove_c_node(&extension.group, &extension.node);
    }

    for run in 0..MAX_ITERATIONS {
        let delta = compact_round(&mut compactor, &extensions, top_left, &node_locks, false);
        if run >= 1 && delta <= EPSILON {
            break;
        }
    }
    compact_round(&mut compactor, &extensions, top_left, &node_locks, true);
    compactor.finish();

    let mut min = KVector::with_values(f64::INFINITY, f64::INFINITY);
    let mut max = KVector::with_values(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for node in graph
        .borrow()
        .c_nodes
        .iter()
        .chain(extensions.iter().map(|e| &e.placeholder))
    {
        let rect = node.borrow().hitbox;
        min.x = min.x.min(rect.x);
        min.y = min.y.min(rect.y);
        max.x = max.x.max(rect.x + rect.width);
        max.y = max.y.max(rect.y + rect.height);
    }
    let offsets = references
        .iter()
        .map(|(reference, old)| {
            let rect = reference.borrow().hitbox;
            KVector::with_values(rect.x - old.x - min.x, rect.y - old.y - min.y)
        })
        .collect();
    let size = KVector::with_values(max.x - min.x, max.y - min.y);
    assert!(
        size.x.is_finite() && size.y.is_finite(),
        "non-finite component compaction"
    );
    (offsets, size)
}

fn remove_node(graph: &CGraphRef, node: &CNodeRef) {
    graph
        .borrow_mut()
        .c_nodes
        .retain(|candidate| !Rc::ptr_eq(candidate, node));
}

fn horizontal(side: PortSide) -> bool {
    matches!(side, PortSide::West | PortSide::East)
}

fn reset_deltas(graph: &CGraphRef) {
    for group in &graph.borrow().c_groups {
        let mut group = group.borrow_mut();
        group.delta = 0.0;
        group.delta_normalized = 0.0;
    }
}

fn compact_round(
    compactor: &mut OneDimensionalCompactor,
    extensions: &[Extension],
    top_left: KVector,
    node_locks: &FxHashMap<usize, [bool; 4]>,
    align: bool,
) -> f64 {
    let graph = compactor.c_graph.clone();
    reset_deltas(&graph);
    let mut delta = 0.0;
    for is_horizontal in [true, false] {
        for extension in extensions {
            if horizontal(extension.side) == is_horizontal {
                let group = extension.placeholder_group.clone();
                graph
                    .borrow_mut()
                    .c_nodes
                    .push(extension.placeholder.clone());
                graph.borrow_mut().c_groups.push(group);
            } else {
                graph.borrow_mut().c_nodes.push(extension.node.clone());
                CGroup::add_c_node(&extension.group, &extension.node);
            }
        }
        compactor.calculate_group_offsets();
        if is_horizontal {
            compactor.force_constraints_recalculation();
        } else {
            delta += graph
                .borrow()
                .c_groups
                .iter()
                .map(|g| g.borrow().delta_normalized.abs())
                .sum::<f64>();
            reset_deltas(&graph);
        }
        let forward = if is_horizontal {
            Direction::Left
        } else {
            Direction::Up
        };
        let backward = if is_horizontal {
            Direction::Right
        } else {
            Direction::Down
        };
        compactor.change_direction(forward);
        if !is_horizontal {
            compactor.force_constraints_recalculation();
        }
        compactor.set_lock_function(None).compact();
        for direction in [backward, forward] {
            compactor.change_direction(direction);
            // Snapshot the Java reposition flags before the solver consumes degrees.
            let frozen: FxHashMap<_, _> = graph
                .borrow()
                .c_nodes
                .iter()
                .map(|node| {
                    let reposition = if align {
                        node_locks
                            .get(&key(node))
                            .is_some_and(|flags| flags[direction_index(direction)])
                    } else {
                        node.borrow().group().unwrap().borrow().out_degree != 0
                    };
                    (key(node), !reposition)
                })
                .collect();
            compactor
                .set_lock_function(Some(Box::new(move |node: &CNodeRef, _: Direction| {
                    frozen.get(&key(node)).copied().unwrap_or(false)
                })))
                .compact();
        }
        compactor
            .set_lock_function(None)
            .change_direction(Direction::Left);
        for extension in extensions {
            if horizontal(extension.side) == is_horizontal {
                remove_node(&graph, &extension.placeholder);
                let group = extension.placeholder_group.clone();
                graph
                    .borrow_mut()
                    .c_groups
                    .retain(|g| !Rc::ptr_eq(g, &group));
            } else {
                remove_node(&graph, &extension.node);
                CGroup::remove_c_node(&extension.group, &extension.node);
            }
        }
        for extension in extensions {
            let movement = extension.group.borrow().delta_normalized;
            if horizontal(extension.side) == is_horizontal {
                let mut node = extension.node.borrow_mut();
                let rect = &mut node.hitbox;
                match extension.side {
                    PortSide::West => {
                        rect.x = top_left.x;
                        rect.width = (rect.width + movement).max(1.0);
                    }
                    PortSide::East => {
                        rect.x += movement;
                        rect.width = (rect.width - movement).max(1.0);
                    }
                    PortSide::North => {
                        rect.y = top_left.y;
                        rect.height = (rect.height + movement).max(1.0);
                    }
                    PortSide::South => {
                        rect.y += movement;
                        rect.height = (rect.height - movement).max(1.0);
                    }
                    PortSide::Undefined => unreachable!(),
                }
            } else {
                let mut node = extension.placeholder.borrow_mut();
                if is_horizontal {
                    node.hitbox.x += movement;
                } else {
                    node.hitbox.y += movement;
                }
            }
        }
    }
    compactor.force_constraints_recalculation();
    delta += graph
        .borrow()
        .c_groups
        .iter()
        .map(|g| g.borrow().delta_normalized.abs())
        .sum::<f64>();
    delta
}
