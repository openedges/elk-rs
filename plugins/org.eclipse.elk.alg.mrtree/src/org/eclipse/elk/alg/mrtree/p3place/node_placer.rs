use std::collections::HashSet;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::mrtree::graph::{TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::mrtree::options::{InternalProperties, MrTreeOptions};
use crate::org::eclipse::elk::alg::mrtree::tree_layout_phases::TreeLayoutPhases;
use crate::org::eclipse::elk::alg::mrtree::tree_util::TreeUtil;

pub struct NodePlacer {
    spacing: f64,
    x_top_adjustment: f64,
    y_top_adjustment: f64,
    direction: Direction,
}

impl Default for NodePlacer {
    fn default() -> Self {
        Self {
            spacing: 0.0,
            x_top_adjustment: 0.0,
            y_top_adjustment: 0.0,
            direction: Direction::Down,
        }
    }
}

impl ILayoutPhase<TreeLayoutPhases, TGraphRef> for NodePlacer {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Processor order nodes", 2.0);

        let (spacing, direction, root) = {
            let mut graph_guard = match graph.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    progress_monitor.done();
                    return;
                }
            };
            let spacing = graph_guard
                .get_property(MrTreeOptions::SPACING_NODE_NODE)
                .unwrap_or(0.0);
            let mut direction = graph_guard
                .get_property(MrTreeOptions::DIRECTION)
                .unwrap_or(Direction::Undefined);
            if direction == Direction::Undefined {
                direction = Direction::Down;
                graph_guard.set_property(MrTreeOptions::DIRECTION, Some(direction));
            }

            let root = graph_guard
                .nodes()
                .iter()
                .find(|node| {
                    node.lock()
                        .ok()
                        .and_then(|mut node_guard| {
                            node_guard.get_property(InternalProperties::ROOT)
                        })
                        .unwrap_or(false)
                })
                .cloned();
            (spacing, direction, root)
        };

        self.spacing = spacing;
        self.direction = direction;

        if let Some(root) = root {
            let mut first_walk_seen: HashSet<usize> = HashSet::new();
            self.first_walk(&root, 0, &mut first_walk_seen);
            progress_monitor.worked(1.0);
            let level_height = root
                .lock()
                .ok()
                .and_then(|mut node_guard| node_guard.get_property(InternalProperties::LEVELHEIGHT))
                .unwrap_or(0.0);
            let mut second_walk_seen: HashSet<usize> = HashSet::new();
            self.second_walk(
                &root,
                self.y_top_adjustment - (level_height / 2.0),
                self.x_top_adjustment,
                &mut second_walk_seen,
            );
            progress_monitor.worked(1.0);
        }

        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &TGraphRef,
    ) -> Option<LayoutProcessorConfiguration<TreeLayoutPhases, TGraphRef>> {
        let mut config = LayoutProcessorConfiguration::create();
        config
            .before(TreeLayoutPhases::P2NodeOrdering)
            .add(std::sync::Arc::new(IntermediateProcessorStrategy::RootProc))
            .before(TreeLayoutPhases::P3NodePlacement)
            .add(std::sync::Arc::new(
                IntermediateProcessorStrategy::LevelHeight,
            ))
            .add(std::sync::Arc::new(
                IntermediateProcessorStrategy::NeighborsProc,
            ))
            .before(TreeLayoutPhases::P4EdgeRouting)
            .add(std::sync::Arc::new(
                IntermediateProcessorStrategy::DirectionProc,
            ))
            .add(std::sync::Arc::new(
                IntermediateProcessorStrategy::NodePositionProc,
            ));
        Some(config)
    }
}

impl NodePlacer {
    fn first_walk(&self, node: &TNodeRef, level: i32, seen: &mut HashSet<usize>) {
        let node_key = std::sync::Arc::as_ptr(node) as usize;
        if !seen.insert(node_key) {
            return;
        }

        let (is_leaf, left_sibling, children) = match node.lock() {
            Ok(mut node_guard) => {
                node_guard.set_property(InternalProperties::MODIFIER, Some(0.0));
                (
                    node_guard.is_leaf(),
                    node_guard
                        .get_property(InternalProperties::LEFTSIBLING)
                        .flatten(),
                    node_guard.children_copy(),
                )
            }
            Err(_) => return,
        };

        if is_leaf {
            let prelim = if let Some(left_sibling) = left_sibling {
                let left_prelim = left_sibling
                    .lock()
                    .ok()
                    .and_then(|mut n| n.get_property(InternalProperties::PRELIM))
                    .unwrap_or(0.0);
                left_prelim + self.spacing + self.mean_node_width(&left_sibling, node)
            } else {
                0.0
            };
            if let Ok(mut node_guard) = node.lock() {
                node_guard.set_property(InternalProperties::PRELIM, Some(prelim));
            }
            return;
        }

        for child in &children {
            self.first_walk(child, level + 1, seen);
        }

        let leftmost = children.first().cloned();
        let rightmost = children.last().cloned();
        let mid_point = match (leftmost.as_ref(), rightmost.as_ref()) {
            (Some(left), Some(right)) => {
                let left_prelim = left
                    .lock()
                    .ok()
                    .and_then(|mut n| n.get_property(InternalProperties::PRELIM))
                    .unwrap_or(0.0);
                let right_prelim = right
                    .lock()
                    .ok()
                    .and_then(|mut n| n.get_property(InternalProperties::PRELIM))
                    .unwrap_or(0.0);
                (right_prelim + left_prelim) / 2.0
            }
            _ => 0.0,
        };

        if let Some(left_sibling) = left_sibling {
            let left_prelim = left_sibling
                .lock()
                .ok()
                .and_then(|mut n| n.get_property(InternalProperties::PRELIM))
                .unwrap_or(0.0);
            let p = left_prelim + self.spacing + self.mean_node_width(&left_sibling, node);
            if let Ok(mut node_guard) = node.lock() {
                node_guard.set_property(InternalProperties::PRELIM, Some(p));
                let modifier = node_guard
                    .get_property(InternalProperties::PRELIM)
                    .unwrap_or(0.0)
                    - mid_point;
                node_guard.set_property(InternalProperties::MODIFIER, Some(modifier));
            }
            self.apportion(node, level);
        } else if let Ok(mut node_guard) = node.lock() {
            node_guard.set_property(InternalProperties::PRELIM, Some(mid_point));
        }
    }

    fn apportion(&self, node: &TNodeRef, _level: i32) {
        let children = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.children_copy())
            .unwrap_or_default();
        let mut leftmost = children.first().cloned();
        let mut neighbor = leftmost
            .as_ref()
            .and_then(|child| child.lock().ok())
            .and_then(|mut child_guard| {
                child_guard
                    .get_property(InternalProperties::LEFTNEIGHBOR)
                    .flatten()
            });

        let mut compare_depth = 1;
        let mut apportion_iterations = 0usize;

        while leftmost.is_some() && neighbor.is_some() {
            apportion_iterations += 1;
            if apportion_iterations > 256 {
                return;
            }

            let mut left_mod_sum = 0.0;
            let mut right_mod_sum = 0.0;
            let mut ancestor_leftmost = leftmost.clone().unwrap();
            let mut ancestor_neighbor = neighbor.clone().unwrap();

            for _ in 0..compare_depth {
                let next_leftmost_parent = ancestor_leftmost
                    .lock()
                    .ok()
                    .and_then(|node_guard| node_guard.parent());
                let next_neighbor_parent = ancestor_neighbor
                    .lock()
                    .ok()
                    .and_then(|node_guard| node_guard.parent());
                let (Some(next_leftmost_parent), Some(next_neighbor_parent)) =
                    (next_leftmost_parent, next_neighbor_parent)
                else {
                    return;
                };
                ancestor_leftmost = next_leftmost_parent;
                ancestor_neighbor = next_neighbor_parent;

                right_mod_sum += ancestor_leftmost
                    .lock()
                    .ok()
                    .and_then(|mut node_guard| {
                        node_guard.get_property(InternalProperties::MODIFIER)
                    })
                    .unwrap_or(0.0);
                left_mod_sum += ancestor_neighbor
                    .lock()
                    .ok()
                    .and_then(|mut node_guard| {
                        node_guard.get_property(InternalProperties::MODIFIER)
                    })
                    .unwrap_or(0.0);
            }

            let neighbor_prelim = neighbor
                .as_ref()
                .and_then(|node| node.lock().ok())
                .and_then(|mut node_guard| node_guard.get_property(InternalProperties::PRELIM))
                .unwrap_or(0.0);
            let leftmost_prelim = leftmost
                .as_ref()
                .and_then(|node| node.lock().ok())
                .and_then(|mut node_guard| node_guard.get_property(InternalProperties::PRELIM))
                .unwrap_or(0.0);
            let mean = self.mean_node_width(leftmost.as_ref().unwrap(), neighbor.as_ref().unwrap());
            let mut move_distance = neighbor_prelim + left_mod_sum + self.spacing + mean
                - leftmost_prelim
                - right_mod_sum;

            if move_distance > 0.0 {
                let mut left_sibling = node.clone();
                let mut left_siblings = 0usize;
                let mut found_ancestor_neighbor = false;
                let mut seen_left_sibling: HashSet<usize> = HashSet::new();
                loop {
                    let key = std::sync::Arc::as_ptr(&left_sibling) as usize;
                    if !seen_left_sibling.insert(key) {
                        return;
                    }
                    if std::sync::Arc::ptr_eq(&left_sibling, &ancestor_neighbor) {
                        found_ancestor_neighbor = true;
                        break;
                    }
                    left_siblings += 1;
                    let next = {
                        let mut current_guard = match left_sibling.lock() {
                            Ok(guard) => guard,
                            Err(_) => return,
                        };
                        current_guard
                            .get_property(InternalProperties::LEFTSIBLING)
                            .flatten()
                    };
                    match next {
                        Some(next) => left_sibling = next,
                        None => break,
                    }
                }

                if !found_ancestor_neighbor || left_siblings == 0 {
                    return;
                }

                let portion = move_distance / left_siblings as f64;
                let mut current = node.clone();
                let mut seen_current: HashSet<usize> = HashSet::new();
                while !std::sync::Arc::ptr_eq(&current, &ancestor_neighbor) {
                    let key = std::sync::Arc::as_ptr(&current) as usize;
                    if !seen_current.insert(key) {
                        return;
                    }
                    let next = {
                        let mut current_guard = match current.lock() {
                            Ok(guard) => guard,
                            Err(_) => return,
                        };
                        let prelim = current_guard
                            .get_property(InternalProperties::PRELIM)
                            .unwrap_or(0.0)
                            + move_distance;
                        current_guard.set_property(InternalProperties::PRELIM, Some(prelim));
                        let modifier = current_guard
                            .get_property(InternalProperties::MODIFIER)
                            .unwrap_or(0.0)
                            + move_distance;
                        current_guard.set_property(InternalProperties::MODIFIER, Some(modifier));
                        current_guard
                            .get_property(InternalProperties::LEFTSIBLING)
                            .flatten()
                    };
                    move_distance -= portion;
                    match next {
                        Some(next) => current = next,
                        None => return,
                    }
                }
            }

            compare_depth += 1;
            if leftmost
                .as_ref()
                .and_then(|node| node.lock().ok())
                .map(|node_guard| node_guard.is_leaf())
                .unwrap_or(false)
            {
                leftmost = TreeUtil::get_left_most(&children, compare_depth);
            } else {
                leftmost = leftmost
                    .as_ref()
                    .and_then(|node| node.lock().ok())
                    .and_then(|node_guard| node_guard.children_copy().first().cloned());
            }
            neighbor = leftmost
                .as_ref()
                .and_then(|node| node.lock().ok())
                .and_then(|mut node_guard| {
                    node_guard
                        .get_property(InternalProperties::LEFTNEIGHBOR)
                        .flatten()
                });
        }
    }

    fn mean_node_width(&self, left: &TNodeRef, right: &TNodeRef) -> f64 {
        let mut width = 0.0;
        if let Ok(left_guard) = left.lock() {
            if self.direction.is_vertical() {
                width += left_guard.size_ref().x / 2.0;
            } else {
                width += left_guard.size_ref().y / 2.0;
            }
        }
        if let Ok(right_guard) = right.lock() {
            if self.direction.is_vertical() {
                width += right_guard.size_ref().x / 2.0;
            } else {
                width += right_guard.size_ref().y / 2.0;
            }
        }
        width
    }

    fn second_walk(&self, node: &TNodeRef, y_coor: f64, mod_sum: f64, seen: &mut HashSet<usize>) {
        let node_key = std::sync::Arc::as_ptr(node) as usize;
        if !seen.insert(node_key) {
            return;
        }

        if let Ok(mut node_guard) = node.lock() {
            let x_temp = node_guard
                .get_property(InternalProperties::PRELIM)
                .unwrap_or(0.0)
                + mod_sum;
            let y_temp = y_coor
                + node_guard
                    .get_property(InternalProperties::LEVELHEIGHT)
                    .unwrap_or(0.0)
                    / 2.0;
            node_guard.set_property(InternalProperties::XCOOR, Some(x_temp.round() as i32));
            node_guard.set_property(InternalProperties::YCOOR, Some(y_temp.round() as i32));

            if !node_guard.is_leaf() {
                if let Some(first_child) = node_guard.children_copy().first().cloned() {
                    self.second_walk(
                        &first_child,
                        y_coor
                            + node_guard
                                .get_property(InternalProperties::LEVELHEIGHT)
                                .unwrap_or(0.0)
                            + self.spacing,
                        mod_sum
                            + node_guard
                                .get_property(InternalProperties::MODIFIER)
                                .unwrap_or(0.0),
                        seen,
                    );
                }
            }

            if let Some(right_sibling) = node_guard
                .get_property(InternalProperties::RIGHTSIBLING)
                .flatten()
            {
                drop(node_guard);
                self.second_walk(&right_sibling, y_coor, mod_sum, seen);
            }
        }
    }
}
