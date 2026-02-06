use std::collections::HashMap;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{IElkProgressMonitor, Pair, Triple};

use crate::org::eclipse::elk::alg::mrtree::graph::{TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::options::{EdgeRoutingMode, InternalProperties, MrTreeOptions};
use crate::org::eclipse::elk::alg::mrtree::tree_util::TreeUtil;

#[derive(Default)]
pub struct CompactionProcessor {
    levels: Vec<Pair<f64, f64>>,
}

impl ILayoutProcessor<TGraphRef> for CompactionProcessor {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Process compaction", 1.0);

        let (enabled, direction, node_node_spacing) = {
            let mut graph_guard = match graph.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    progress_monitor.done();
                    return;
                }
            };
            let enabled = graph_guard.get_property(MrTreeOptions::COMPACTION).unwrap_or(false);
            let direction = graph_guard
                .get_property(MrTreeOptions::DIRECTION)
                .unwrap_or(Direction::Undefined);
            let spacing = graph_guard
                .get_property(MrTreeOptions::SPACING_NODE_NODE)
                .unwrap_or(0.0);
            (enabled, direction, spacing)
        };

        if !enabled {
            progress_monitor.done();
            return;
        }

        self.set_up_levels(graph, direction);
        self.compute_node_constraints(graph, node_node_spacing / 4.0);

        let nodes = {
            let graph_guard = match graph.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    progress_monitor.done();
                    return;
                }
            };
            graph_guard.nodes().clone()
        };

        let mut nodes_sorted = nodes.clone();
        let dir_vec = TreeUtil::get_direction_vector(direction);
        nodes_sorted.sort_by(|a, b| {
            let a_pos = a
                .lock()
                .ok()
                .map(|node| *node.position_ref())
                .unwrap_or_default();
            let b_pos = b
                .lock()
                .ok()
                .map(|node| *node.position_ref())
                .unwrap_or_default();
            dir_vec
                .dot_product(&a_pos)
                .partial_cmp(&dir_vec.dot_product(&b_pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for node in nodes_sorted {
            let is_root = node
                .lock()
                .ok()
                .and_then(|mut node_guard| node_guard.get_property(InternalProperties::ROOT))
                .unwrap_or(false);
            if is_root {
                continue;
            }

            let (dependent, parent) = (
                self.get_lowest_dependent_node(&node, direction),
                TreeUtil::get_lowest_parent(&node, graph, direction),
            );

            let mut new_pos = 0.0;
            let mut new_pos_size = 0.0;
            let size = node
                .lock()
                .ok()
                .map(|n| *n.size_ref())
                .unwrap_or_default();

            if let Some(dep) = dependent.clone() {
                let dep_pos = dep
                    .lock()
                    .ok()
                    .map(|n| *n.position_ref())
                    .unwrap_or_default();
                let dep_size = dep
                    .lock()
                    .ok()
                    .map(|n| *n.size_ref())
                    .unwrap_or_default();
                if let Some(parent) = parent.clone() {
                    let parent_pos = parent
                        .lock()
                        .ok()
                        .map(|n| *n.position_ref())
                        .unwrap_or_default();
                    let parent_size = parent
                        .lock()
                        .ok()
                        .map(|n| *n.size_ref())
                        .unwrap_or_default();
                    match direction {
                        Direction::Left => {
                            new_pos = dep_pos.x - node_node_spacing - size.x;
                            let alt = parent_pos.x - node_node_spacing - size.x;
                            if alt < new_pos {
                                new_pos = alt;
                            }
                            new_pos_size = new_pos + size.x;
                        }
                        Direction::Right => {
                            new_pos = dep_pos.x + dep_size.x + node_node_spacing;
                            let alt = parent_pos.x + parent_size.x + node_node_spacing;
                            if alt > new_pos {
                                new_pos = alt;
                            }
                            new_pos_size = new_pos + size.x;
                        }
                        Direction::Up => {
                            new_pos = dep_pos.y - node_node_spacing - size.y;
                            let alt = parent_pos.y - node_node_spacing - size.y;
                            if alt < new_pos {
                                new_pos = alt;
                            }
                            new_pos_size = new_pos + size.y;
                        }
                        Direction::Down | Direction::Undefined => {
                            new_pos = dep_pos.y + dep_size.y + node_node_spacing;
                            let alt = parent_pos.y + parent_size.y + node_node_spacing;
                            if alt > new_pos {
                                new_pos = alt;
                            }
                            new_pos_size = new_pos + size.y;
                        }
                    }
                }
            } else if let Some(parent) = parent.clone() {
                let parent_pos = parent
                    .lock()
                    .ok()
                    .map(|n| *n.position_ref())
                    .unwrap_or_default();
                let parent_size = parent
                    .lock()
                    .ok()
                    .map(|n| *n.size_ref())
                    .unwrap_or_default();
                match direction {
                    Direction::Left => {
                        new_pos = parent_pos.x - node_node_spacing - size.x;
                        new_pos_size = new_pos + size.x;
                    }
                    Direction::Right => {
                        new_pos = parent_pos.x + parent_size.x + node_node_spacing;
                        new_pos_size = new_pos + size.x;
                    }
                    Direction::Up => {
                        new_pos = parent_pos.y - node_node_spacing - size.y;
                        new_pos_size = new_pos + size.y;
                    }
                    Direction::Down | Direction::Undefined => {
                        new_pos = parent_pos.y + parent_size.y + node_node_spacing;
                        new_pos_size = new_pos + size.y;
                    }
                }
            }

            let routing_mode = graph
                .lock()
                .ok()
                .and_then(|mut g| g.get_property(MrTreeOptions::EDGE_ROUTING_MODE))
                .unwrap_or(EdgeRoutingMode::AvoidOverlap);

            if routing_mode == EdgeRoutingMode::AvoidOverlap {
                let level = self
                    .levels
                    .iter()
                    .position(|pair| pair.first() <= &new_pos && pair.second() >= &new_pos_size);

                let chosen_level = if let Some(index) = level {
                    Some((index, self.levels[index].clone()))
                } else if direction == Direction::Left || direction == Direction::Up {
                    self.levels
                        .iter()
                        .enumerate()
                        .skip(1)
                        .find(|(_, pair)| pair.first() <= &new_pos)
                        .map(|(idx, pair)| (idx, pair.clone()))
                } else {
                    self.levels
                        .iter()
                        .enumerate()
                        .skip(1)
                        .find(|(_, pair)| pair.first() >= &new_pos)
                        .map(|(idx, pair)| (idx, pair.clone()))
                };

                if let Some((index, level_pair)) = chosen_level {
                    if let Ok(mut node_guard) = node.lock() {
                        if direction.is_horizontal() {
                            node_guard.position().x = *level_pair.first();
                        } else {
                            node_guard.position().y = *level_pair.first();
                        }
                        let current_level = node_guard
                            .get_property(MrTreeOptions::TREE_LEVEL)
                            .unwrap_or(0);
                        if index > 0 && current_level != index as i32 {
                            node_guard.set_property(InternalProperties::COMPACT_LEVEL_ASCENSION, Some(true));
                            node_guard.set_property(MrTreeOptions::TREE_LEVEL, Some(index as i32));
                        }
                    }
                }
            } else if let Ok(mut node_guard) = node.lock() {
                if direction.is_horizontal() {
                    node_guard.position().x = new_pos;
                } else {
                    node_guard.position().y = new_pos;
                }
            }
        }

        progress_monitor.done();
    }
}

impl CompactionProcessor {
    fn set_up_levels(&mut self, graph: &TGraphRef, direction: Direction) {
        self.levels.clear();

        let nodes = {
            let graph_guard = match graph.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            graph_guard.nodes().clone()
        };

        for node in nodes {
            if let Ok(mut node_guard) = node.lock() {
                let level = node_guard.get_property(MrTreeOptions::TREE_LEVEL).unwrap_or(0) as usize;
                while level >= self.levels.len() {
                    self.levels.push(Pair::of(f64::MAX, -f64::MAX));
                }
                let pos = node_guard.position_ref();
                let size = node_guard.size_ref();
                if direction.is_horizontal() {
                    if pos.x < *self.levels[level].first() {
                        self.levels[level].set_first(pos.x);
                    }
                    if pos.x + size.x > *self.levels[level].second() {
                        self.levels[level].set_second(pos.x + size.x);
                    }
                } else {
                    if pos.y < *self.levels[level].first() {
                        self.levels[level].set_first(pos.y);
                    }
                    if pos.y + size.y > *self.levels[level].second() {
                        self.levels[level].set_second(pos.y + size.y);
                    }
                }
            }
        }
    }

    fn compute_node_constraints(&mut self, graph: &TGraphRef, node_node_spacing: f64) {
        let direction = graph
            .lock()
            .ok()
            .and_then(|mut g| g.get_property(MrTreeOptions::DIRECTION))
            .unwrap_or(Direction::Undefined);
        let right = if direction.is_horizontal() {
            Direction::Down
        } else {
            Direction::Right
        };

        let nodes = {
            let graph_guard = match graph.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            graph_guard.nodes().clone()
        };

        let actual_nodes: Vec<TNodeRef> = nodes
            .into_iter()
            .filter(|node| {
                node.lock()
                    .ok()
                    .and_then(|node_guard| node_guard.label().map(|label| label.contains("SUPER_ROOT")))
                    .map(|contains| !contains)
                    .unwrap_or(true)
            })
            .collect();

        let mut points: Vec<Triple<TNodeRef, KVector, bool>> = actual_nodes
            .iter()
            .filter_map(|node| {
                node.lock().ok().map(|node_guard| {
                    let mut pos = *node_guard.position_ref();
                    pos.sub_values(node_node_spacing, node_node_spacing);
                    Triple::new(node.clone(), pos, true)
                })
            })
            .collect();

        points.extend(actual_nodes.iter().filter_map(|node| {
            node.lock().ok().map(|node_guard| {
                let mut pos = *node_guard.position_ref();
                pos.add_values(
                    node_guard.size_ref().x + node_node_spacing,
                    node_guard.size_ref().y + node_node_spacing,
                );
                Triple::new(node.clone(), pos, false)
            })
        }));

        let right_vec = TreeUtil::get_direction_vector(right);
        points.sort_by(|a, b| {
            right_vec
                .dot_product(a.second())
                .partial_cmp(&right_vec.dot_product(b.second()))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut active: Vec<TNodeRef> = Vec::new();
        let mut candidates: HashMap<usize, TNodeRef> = HashMap::new();

        for point in points {
            let node = point.first().clone();
            if *point.third() {
                insert_sorted(&mut active, &node, direction);
                if let Some(left) = left_neighbor(&active, &node) {
                    candidates.insert(node_key(&node), left);
                }
                if let Some(right_neighbor) = right_neighbor(&active, &node) {
                    candidates.insert(node_key(&right_neighbor), node.clone());
                }
            } else {
                if let Some(left) = left_neighbor(&active, &node) {
                    if candidates
                        .get(&node_key(&node))
                        .map(|cand| std::sync::Arc::ptr_eq(cand, &left))
                        .unwrap_or(false)
                    {
                        push_constraint(&node, &left);
                    }
                }
                if let Some(right) = right_neighbor(&active, &node) {
                    if candidates
                        .get(&node_key(&right))
                        .map(|cand| std::sync::Arc::ptr_eq(cand, &node))
                        .unwrap_or(false)
                    {
                        push_constraint(&right, &node);
                    }
                }
                active.retain(|n| !std::sync::Arc::ptr_eq(n, &node));
            }
        }
    }

    fn get_lowest_dependent_node(&self, node: &TNodeRef, direction: Direction) -> Option<TNodeRef> {
        let constraints = node
            .lock()
            .ok()
            .and_then(|mut node_guard| node_guard.get_property(InternalProperties::COMPACT_CONSTRAINTS));
        let constraints = constraints.unwrap_or_default();
        if constraints.is_empty() {
            return None;
        }
        if constraints.len() == 1 {
            return constraints.first().cloned();
        }

        let mut best: Option<TNodeRef> = None;
        for candidate in constraints {
            let candidate_pos = candidate
                .lock()
                .ok()
                .map(|n| *n.position_ref())
                .unwrap_or_default();
            let candidate_size = candidate
                .lock()
                .ok()
                .map(|n| *n.size_ref())
                .unwrap_or_default();
            let value = match direction {
                Direction::Left => candidate_pos.x,
                Direction::Right => candidate_pos.x + candidate_size.x,
                Direction::Up => candidate_pos.y,
                Direction::Down | Direction::Undefined => candidate_pos.y + candidate_size.y,
            };
            best = match best {
                None => Some(candidate),
                Some(current_best) => {
                    let current_pos = current_best
                        .lock()
                        .ok()
                        .map(|n| *n.position_ref())
                        .unwrap_or_default();
                    let current_size = current_best
                        .lock()
                        .ok()
                        .map(|n| *n.size_ref())
                        .unwrap_or_default();
                    let current_value = match direction {
                        Direction::Left => current_pos.x,
                        Direction::Right => current_pos.x + current_size.x,
                        Direction::Up => current_pos.y,
                        Direction::Down | Direction::Undefined => current_pos.y + current_size.y,
                    };
                    if (direction == Direction::Left || direction == Direction::Up)
                        && value < current_value
                        || (direction == Direction::Right || direction == Direction::Down || direction == Direction::Undefined)
                            && value > current_value
                    {
                        Some(candidate)
                    } else {
                        Some(current_best)
                    }
                }
            };
        }
        best
    }
}

fn node_key(node: &TNodeRef) -> usize {
    std::sync::Arc::as_ptr(node) as usize
}

fn insert_sorted(active: &mut Vec<TNodeRef>, node: &TNodeRef, direction: Direction) {
    let dir_vec = TreeUtil::get_direction_vector(direction);
    let node_pos = node
        .lock()
        .ok()
        .map(|n| *n.position_ref())
        .unwrap_or_default();
    let node_proj = dir_vec.dot_product(&node_pos);
    let index = active
        .iter()
        .position(|other| {
            let other_pos = other
                .lock()
                .ok()
                .map(|n| *n.position_ref())
                .unwrap_or_default();
            node_proj < dir_vec.dot_product(&other_pos)
        })
        .unwrap_or(active.len());
    active.insert(index, node.clone());
}

fn left_neighbor(active: &[TNodeRef], node: &TNodeRef) -> Option<TNodeRef> {
    let index = active.iter().position(|n| std::sync::Arc::ptr_eq(n, node))?;
    if index == 0 {
        None
    } else {
        Some(active[index - 1].clone())
    }
}

fn right_neighbor(active: &[TNodeRef], node: &TNodeRef) -> Option<TNodeRef> {
    let index = active.iter().position(|n| std::sync::Arc::ptr_eq(n, node))?;
    if index + 1 >= active.len() {
        None
    } else {
        Some(active[index + 1].clone())
    }
}

fn push_constraint(node: &TNodeRef, constraint: &TNodeRef) {
        if let Ok(mut node_guard) = node.lock() {
            let mut list = node_guard
                .get_property(InternalProperties::COMPACT_CONSTRAINTS)
                .unwrap_or_else(Vec::new);
            list.push(constraint.clone());
            node_guard.set_property(InternalProperties::COMPACT_CONSTRAINTS, Some(list));
        }
}
