use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{IElkProgressMonitor, Pair};

use crate::org::eclipse::elk::alg::mrtree::graph::{TEdgeRef, TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::mrtree::options::{
    EdgeRoutingMode, InternalProperties, MrTreeOptions,
};
use crate::org::eclipse::elk::alg::mrtree::p4route::MultiLevelEdgeNodeNodeGap;
use crate::org::eclipse::elk::alg::mrtree::tree_layout_phases::TreeLayoutPhases;
use crate::org::eclipse::elk::alg::mrtree::tree_util::TreeUtil;

/// Pre-extracted node data for SoA access — single lock per node.
struct NodeData {
    level: i32,
    pos: KVector,
    size: KVector,
    level_min: f64,
    level_max: f64,
    is_super_root: bool,
}

/// Pre-computed aggregate graph statistics from node data.
struct GraphStats {
    avg_center_h: f64,
    avg_center_v: f64,
    max_extent_h: f64,
    max_extent_v: f64,
    min_extent_h: f64,
    min_extent_v: f64,
}

impl GraphStats {
    fn average_center(&self, horizontal: bool) -> f64 {
        if horizontal {
            self.avg_center_h
        } else {
            self.avg_center_v
        }
    }

    fn max_node_extent(&self, horizontal: bool, padding: f64) -> f64 {
        if horizontal {
            self.max_extent_h + padding
        } else {
            self.max_extent_v + padding
        }
    }

    fn min_node_extent(&self, horizontal: bool, padding: f64) -> f64 {
        if horizontal {
            self.min_extent_h - padding
        } else {
            self.min_extent_v - padding
        }
    }
}

#[derive(Default)]
pub struct EdgeRouter;

impl EdgeRouter {
    const ONE_HALF: f64 = 0.5;
    const STEEP_END_EDGE_THRESHOLD_DISTANCE: f64 = 50.0;
    const STEEP_END_EDGE_RATIO: f64 = 5.3;
    const STEEP_END_EDGE_SAMPLE_HEIGHT: f64 = 40.0;
}

impl ILayoutPhase<TreeLayoutPhases, TGraphRef> for EdgeRouter {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Edge routing", 1.0);

        let mode = {
            let mut graph_guard = match graph.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    progress_monitor.done();
                    return;
                }
            };
            graph_guard
                .get_property(MrTreeOptions::EDGE_ROUTING_MODE)
                .unwrap_or(EdgeRoutingMode::AvoidOverlap)
        };

        match mode {
            EdgeRoutingMode::None => {}
            EdgeRoutingMode::MiddleToMiddle => self.middle_to_middle(graph),
            EdgeRoutingMode::AvoidOverlap => {
                self.avoid_overlap(graph);
                let edges = graph
                    .lock()
                    .ok()
                    .map(|g| g.edges().clone())
                    .unwrap_or_default();
                for edge in edges {
                    if edge
                        .lock()
                        .ok()
                        .map(|guard| guard.bend_points_ref().len())
                        .unwrap_or(0)
                        < 2
                    {
                        self.middle_to_middle_edge_route(&edge);
                    }
                }
            }
        }

        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &TGraphRef,
    ) -> Option<LayoutProcessorConfiguration<TreeLayoutPhases, TGraphRef>> {
        let mut config = LayoutProcessorConfiguration::create();
        config
            .before(TreeLayoutPhases::P4EdgeRouting)
            .add(Arc::new(IntermediateProcessorStrategy::LevelCoords))
            .add(Arc::new(IntermediateProcessorStrategy::CompactionProc))
            .add(Arc::new(IntermediateProcessorStrategy::GraphBoundsProc));
        Some(config)
    }
}

/// Build SoA node data map and aggregate graph stats from all nodes — single lock per node.
fn build_node_data(
    nodes: &[TNodeRef],
) -> (HashMap<usize, NodeData>, GraphStats) {
    let mut data = HashMap::with_capacity(nodes.len());
    let mut sum_h = 0.0_f64;
    let mut sum_v = 0.0_f64;
    let mut max_h = f64::MIN;
    let mut max_v = f64::MIN;
    let mut min_h = f64::MAX;
    let mut min_v = f64::MAX;
    let mut count = 0usize;

    for node in nodes {
        let key = Arc::as_ptr(node) as usize;
        if let Ok(mut guard) = node.lock() {
            let level = guard
                .get_property(MrTreeOptions::TREE_LEVEL)
                .unwrap_or(0);
            let pos = *guard.position_ref();
            let size = *guard.size_ref();
            let level_min = guard
                .get_property(InternalProperties::LEVELMIN)
                .unwrap_or(0.0);
            let level_max = guard
                .get_property(InternalProperties::LEVELMAX)
                .unwrap_or(0.0);
            let is_super_root = guard
                .label()
                .map(|label| label == "SUPER_ROOT")
                .unwrap_or(false);

            sum_h += pos.y + size.y / 2.0;
            sum_v += pos.x + size.x / 2.0;
            let ext_h = pos.y + size.y;
            let ext_v = pos.x + size.x;
            if ext_h > max_h {
                max_h = ext_h;
            }
            if ext_v > max_v {
                max_v = ext_v;
            }
            if pos.y < min_h {
                min_h = pos.y;
            }
            if pos.x < min_v {
                min_v = pos.x;
            }
            count += 1;

            data.insert(
                key,
                NodeData {
                    level,
                    pos,
                    size,
                    level_min,
                    level_max,
                    is_super_root,
                },
            );
        }
    }

    let stats = if count > 0 {
        GraphStats {
            avg_center_h: sum_h / count as f64,
            avg_center_v: sum_v / count as f64,
            max_extent_h: max_h,
            max_extent_v: max_v,
            min_extent_h: min_h,
            min_extent_v: min_v,
        }
    } else {
        GraphStats {
            avg_center_h: 0.0,
            avg_center_v: 0.0,
            max_extent_h: 0.0,
            max_extent_v: 0.0,
            min_extent_h: 0.0,
            min_extent_v: 0.0,
        }
    };

    (data, stats)
}

#[inline]
fn nd_level(nd: &HashMap<usize, NodeData>, node: &TNodeRef) -> i32 {
    nd.get(&(Arc::as_ptr(node) as usize))
        .map(|d| d.level)
        .unwrap_or(0)
}

#[inline]
fn nd_pos(nd: &HashMap<usize, NodeData>, node: &TNodeRef) -> KVector {
    nd.get(&(Arc::as_ptr(node) as usize))
        .map(|d| d.pos)
        .unwrap_or_default()
}

#[inline]
fn nd_size(nd: &HashMap<usize, NodeData>, node: &TNodeRef) -> KVector {
    nd.get(&(Arc::as_ptr(node) as usize))
        .map(|d| d.size)
        .unwrap_or_default()
}

#[inline]
fn nd_level_min(nd: &HashMap<usize, NodeData>, node: &TNodeRef) -> f64 {
    nd.get(&(Arc::as_ptr(node) as usize))
        .map(|d| d.level_min)
        .unwrap_or(0.0)
}

#[inline]
fn nd_level_max(nd: &HashMap<usize, NodeData>, node: &TNodeRef) -> f64 {
    nd.get(&(Arc::as_ptr(node) as usize))
        .map(|d| d.level_max)
        .unwrap_or(0.0)
}

#[inline]
fn nd_is_super_root(nd: &HashMap<usize, NodeData>, node: &TNodeRef) -> bool {
    nd.get(&(Arc::as_ptr(node) as usize))
        .map(|d| d.is_super_root)
        .unwrap_or(false)
}

impl EdgeRouter {
    fn middle_to_middle(&self, graph: &TGraphRef) {
        let edges = graph
            .lock()
            .ok()
            .map(|g| g.edges().clone())
            .unwrap_or_default();
        for edge in edges {
            self.middle_to_middle_edge_route(&edge);
        }
    }

    fn middle_to_middle_edge_route(&self, edge: &TEdgeRef) {
        let (source, target) = {
            let edge_guard = match edge.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            (edge_guard.source(), edge_guard.target())
        };
        let (Some(source), Some(target)) = (source, target) else {
            return;
        };

        let (source_point, target_point, source_size, target_size) = {
            let source_guard = source.lock().ok();
            let target_guard = target.lock().ok();
            let Some(source_guard) = source_guard else {
                return;
            };
            let Some(target_guard) = target_guard else {
                return;
            };
            let source_pos = source_guard.position_ref();
            let source_size = *source_guard.size_ref();
            let target_pos = target_guard.position_ref();
            let target_size = *target_guard.size_ref();
            (
                KVector::with_values(
                    source_pos.x + source_size.x / 2.0,
                    source_pos.y + source_size.y / 2.0,
                ),
                KVector::with_values(
                    target_pos.x + target_size.x / 2.0,
                    target_pos.y + target_size.y / 2.0,
                ),
                source_size,
                target_size,
            )
        };

        if let Ok(mut edge_guard) = edge.lock() {
            let bend_points = edge_guard.bend_points();
            bend_points.insert(0, source_point);
            bend_points.add_vector(target_point);

            if bend_points.len() >= 2 {
                let mut source_border = bend_points.get(0);
                let next = bend_points.get(1);
                TreeUtil::to_node_border(&mut source_border, &next, &source_size);
                bend_points.set(0, source_border);

                let last_index = bend_points.len() - 1;
                let mut target_border = bend_points.get(last_index);
                let prev = bend_points.get(last_index - 1);
                TreeUtil::to_node_border(&mut target_border, &prev, &target_size);
                bend_points.set(last_index, target_border);
            }
        }
    }

    fn avoid_overlap(&self, graph: &TGraphRef) {
        let (node_bendpoint_padding, edge_end_texture_padding, direction) = {
            let mut graph_guard = match graph.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            (
                graph_guard
                    .get_property(MrTreeOptions::SPACING_EDGE_NODE)
                    .unwrap_or(0.0),
                graph_guard
                    .get_property(MrTreeOptions::EDGE_END_TEXTURE_LENGTH)
                    .unwrap_or(0.0),
                graph_guard
                    .get_property(MrTreeOptions::DIRECTION)
                    .unwrap_or(Direction::Down),
            )
        };

        // SoA: pre-extract all node data + graph stats in single pass
        let nodes = graph
            .lock()
            .ok()
            .map(|g| g.nodes().clone())
            .unwrap_or_default();
        let (nd, stats) = build_node_data(&nodes);

        // Cache graph bounds once
        let graph_bounds = graph
            .lock()
            .ok()
            .map(|mut guard| {
                let xmin = guard
                    .get_property(InternalProperties::GRAPH_XMIN)
                    .unwrap_or(0.0);
                let xmax = guard
                    .get_property(InternalProperties::GRAPH_XMAX)
                    .unwrap_or(0.0);
                let ymin = guard
                    .get_property(InternalProperties::GRAPH_YMIN)
                    .unwrap_or(0.0);
                let ymax = guard
                    .get_property(InternalProperties::GRAPH_YMAX)
                    .unwrap_or(0.0);
                (xmin, xmax, ymin, ymax)
            })
            .unwrap_or((0.0, 0.0, 0.0, 0.0));

        self.avoid_overlap_set_start_points(graph, direction, node_bendpoint_padding, &nd);
        self.avoid_overlap_special_edges(
            graph,
            direction,
            node_bendpoint_padding,
            edge_end_texture_padding,
            &nd,
            &stats,
            graph_bounds,
        );
        self.avoid_overlap_set_end_points(
            graph,
            direction,
            node_bendpoint_padding,
            edge_end_texture_padding,
            &nd,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn avoid_overlap_special_edges(
        &self,
        graph: &TGraphRef,
        direction: Direction,
        node_bendpoint_padding: f64,
        edge_end_texture_padding: f64,
        nd: &HashMap<usize, NodeData>,
        stats: &GraphStats,
        graph_bounds: (f64, f64, f64, f64),
    ) {
        let mut side_one_edges = 0;
        let mut side_two_edges = 0;
        let mut node_gaps: HashMap<u64, MultiLevelEdgeNodeNodeGap> = HashMap::new();

        let nodes = graph
            .lock()
            .ok()
            .map(|g| g.nodes().clone())
            .unwrap_or_default();
        let max_level = nodes.iter().map(|n| nd_level(nd, n)).max().unwrap_or(0) + 1;
        let mut outs_per_level = vec![0_i32; max_level.max(0) as usize];
        let mut ins_per_level = vec![0_i32; max_level.max(0) as usize];

        let edges = graph
            .lock()
            .ok()
            .map(|g| g.edges().clone())
            .unwrap_or_default();
        let mut seen: HashSet<usize> = HashSet::new();
        for edge in edges
            .into_iter()
            .filter(|edge| seen.insert(Arc::as_ptr(edge) as usize))
        {
            let (source, target) = match edge.lock().ok() {
                Some(guard) => (guard.source(), guard.target()),
                None => (None, None),
            };
            let (Some(source), Some(target)) = (source, target) else {
                continue;
            };

            let source_level = nd_level(nd, &source);
            let target_level = nd_level(nd, &target);
            let level_diff = target_level - source_level;
            if level_diff > 1 {
                // Track last bend point locally to avoid re-locking edge per level
                let mut has_bends = false;
                let source_pos = nd_pos(nd, &source);
                let target_pos = nd_pos(nd, &target);
                for cur_level in (source_level + 1)..target_level {
                    let mut next_level_nodes: Vec<TNodeRef> = nodes
                        .iter()
                        .filter(|node| nd_level(nd, node) == cur_level)
                        .cloned()
                        .collect();

                    if direction.is_horizontal() {
                        next_level_nodes
                            .sort_by(|a, b| {
                                nd_pos(nd, a)
                                    .y
                                    .partial_cmp(&nd_pos(nd, b).y)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                    } else {
                        next_level_nodes
                            .sort_by(|a, b| {
                                nd_pos(nd, a)
                                    .x
                                    .partial_cmp(&nd_pos(nd, b).x)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                    }

                    let interpolation =
                        (cur_level - source_level) as f64 / (target_level - source_level) as f64;
                    let mut index = 0usize;
                    while index < next_level_nodes.len() {
                        let pos = nd_pos(nd, &next_level_nodes[index]);
                        let projection = if direction.is_horizontal() {
                            source_pos.y * (1.0 - interpolation) + target_pos.y * interpolation
                        } else {
                            source_pos.x * (1.0 - interpolation) + target_pos.x * interpolation
                        };
                        if (if direction.is_horizontal() {
                            pos.y
                        } else {
                            pos.x
                        }) > projection
                        {
                            break;
                        }
                        index += 1;
                    }

                    if !next_level_nodes.is_empty() {
                        // Use locally tracked state instead of locking edge
                        let start = if has_bends {
                            KVector::new() // matches what get_last() would return for placeholder
                        } else {
                            source_pos
                        };

                        let last_node = next_level_nodes.last().unwrap();
                        let first_node = next_level_nodes.first().unwrap();
                        let last_pos = nd_pos(nd, last_node);
                        let last_size = nd_size(nd, last_node);
                        let last = KVector::with_values(
                            last_pos.x + last_size.x,
                            last_pos.y + last_size.y,
                        );
                        let first_pos = nd_pos(nd, first_node);
                        let first_size = nd_size(nd, first_node);
                        let first = KVector::with_values(
                            first_pos.x + first_size.x,
                            first_pos.y + first_size.y,
                        );

                        if direction.is_horizontal() {
                            if index >= next_level_nodes.len().saturating_sub(1)
                                && start.y > last.y
                                && target_pos.y > last.y
                            {
                                continue;
                            }
                            if index == 0 && start.y < first.x && target_pos.y < first.y {
                                continue;
                            }
                        } else {
                            if index >= next_level_nodes.len().saturating_sub(1)
                                && start.x > last.x
                                && target_pos.x > last.x
                            {
                                continue;
                            }
                            if index == 0 && start.x < first.x && target_pos.x < first.x {
                                continue;
                            }
                        }
                    }

                    let (first_index, second_index) = match edge.lock().ok() {
                        Some(mut guard) => {
                            let bends = guard.bend_points();
                            let first_index = bends.len();
                            bends.add_vector(KVector::new());
                            let second_index = bends.len();
                            bends.add_vector(KVector::new());
                            (Some(first_index), Some(second_index))
                        }
                        None => (None, None),
                    };
                    let (Some(first_index), Some(second_index)) = (first_index, second_index)
                    else {
                        continue;
                    };
                    has_bends = true;

                    let key = TreeUtil::get_unique_long(cur_level, index as i32);
                    let neighbor_one = if index == 0 {
                        None
                    } else {
                        Some(next_level_nodes[index - 1].clone())
                    };
                    let neighbor_two = if index >= next_level_nodes.len() {
                        None
                    } else {
                        Some(next_level_nodes[index].clone())
                    };

                    if let Some(gap) = node_gaps.get_mut(&key) {
                        gap.add_bend_points(edge.clone(), first_index, second_index);
                    } else {
                        let gap = MultiLevelEdgeNodeNodeGap::new(
                            neighbor_one,
                            neighbor_two,
                            edge.clone(),
                            first_index,
                            second_index,
                            graph.clone(),
                        );
                        node_gaps.insert(key, gap);
                    }

                    let (graph_xmin, graph_xmax, graph_ymin, graph_ymax) = graph_bounds;
                    if let Some(gap) = node_gaps.get(&key) {
                        if !direction.is_horizontal() {
                            if gap.is_on_first_node_side() {
                                if let Some(neighbor_two) = gap.neighbor_two() {
                                    let pos = nd_pos(nd, &neighbor_two);
                                    if pos.x <= graph_xmin {
                                        side_one_edges += 1;
                                    }
                                }
                            }
                            if gap.is_on_last_node_side() {
                                if let Some(neighbor_one) = gap.neighbor_one() {
                                    let pos = nd_pos(nd, &neighbor_one);
                                    let size = nd_size(nd, &neighbor_one);
                                    if pos.x + size.x >= graph_xmax {
                                        side_two_edges += 1;
                                    }
                                }
                            }
                        } else {
                            if gap.is_on_first_node_side() {
                                if let Some(neighbor_two) = gap.neighbor_two() {
                                    let pos = nd_pos(nd, &neighbor_two);
                                    if pos.y <= graph_ymin {
                                        side_one_edges += 1;
                                    }
                                }
                            }
                            if gap.is_on_last_node_side() {
                                if let Some(neighbor_one) = gap.neighbor_one() {
                                    let pos = nd_pos(nd, &neighbor_one);
                                    let size = nd_size(nd, &neighbor_one);
                                    if pos.y + size.y >= graph_ymax {
                                        side_two_edges += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            } else if level_diff == 0 {
                self.middle_to_middle_edge_route(&edge);
            } else if level_diff < 0 {
                let source_level_usize = source_level.max(0) as usize;
                let target_level_usize = target_level.max(0) as usize;
                if source_level_usize < outs_per_level.len() {
                    outs_per_level[source_level_usize] += 1;
                }
                if target_level_usize < ins_per_level.len() {
                    ins_per_level[target_level_usize] += 1;
                }

                let side_edges = Pair::of(side_one_edges, side_two_edges);
                let in_outs = Pair::of(
                    ins_per_level.get(target_level_usize).copied().unwrap_or(0),
                    outs_per_level.get(source_level_usize).copied().unwrap_or(0),
                );
                let updated = self.avoid_overlap_handle_cycle_inducing_edges(
                    &edge,
                    direction,
                    side_edges,
                    node_bendpoint_padding,
                    edge_end_texture_padding,
                    in_outs,
                    nd,
                    stats,
                );
                side_one_edges = *updated.first();
                side_two_edges = *updated.second();
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn avoid_overlap_handle_cycle_inducing_edges(
        &self,
        edge: &TEdgeRef,
        direction: Direction,
        side_edges: Pair<i32, i32>,
        node_bendpoint_padding: f64,
        edge_end_texture_padding: f64,
        in_outs: Pair<i32, i32>,
        nd: &HashMap<usize, NodeData>,
        stats: &GraphStats,
    ) -> Pair<i32, i32>
    {
        let mut side_one_edges = *side_edges.first();
        let mut side_two_edges = *side_edges.second();

        let (source, target) = match edge.lock().ok() {
            Some(guard) => (guard.source(), guard.target()),
            None => (None, None),
        };
        let (Some(source), Some(target)) = (source, target) else {
            return Pair::of(side_one_edges, side_two_edges);
        };

        let source_pos = nd_pos(nd, &source);
        let source_size = nd_size(nd, &source);
        let target_pos = nd_pos(nd, &target);
        let target_size = nd_size(nd, &target);

        let bend_tmp = if direction.is_horizontal() {
            let middle_tree = stats.average_center(true);
            if source_pos.y + source_size.y / 2.0 > middle_tree {
                side_two_edges += 1;
                stats.max_node_extent(true, node_bendpoint_padding * side_two_edges as f64)
            } else {
                side_one_edges += 1;
                stats.min_node_extent(true, node_bendpoint_padding * side_one_edges as f64)
            }
        } else {
            let middle_tree = stats.average_center(false);
            if source_pos.x + source_size.x / 2.0 > middle_tree {
                side_two_edges += 1;
                stats.max_node_extent(false, node_bendpoint_padding * side_two_edges as f64)
            } else {
                side_one_edges += 1;
                stats.min_node_extent(false, node_bendpoint_padding * side_one_edges as f64)
            }
        };

        if let Ok(mut edge_guard) = edge.lock() {
            let bends = edge_guard.bend_points();
            if direction == Direction::Left {
                let level_min = nd_level_min(nd, &source);
                bends.add_values(level_min - node_bendpoint_padding, bend_tmp);
                bends.add_values(
                    target_pos.x
                        + target_size.x
                        + node_bendpoint_padding
                        + edge_end_texture_padding,
                    bend_tmp,
                );
                bends.add_values(
                    target_pos.x
                        + target_size.x
                        + node_bendpoint_padding
                        + edge_end_texture_padding,
                    target_pos.y + target_size.y / 2.0,
                );
                bends.add_values(
                    target_pos.x + target_size.x,
                    target_pos.y + target_size.y / 2.0,
                );
            } else if direction == Direction::Right {
                let level_max = nd_level_max(nd, &source);
                bends.add_values(
                    level_max + node_bendpoint_padding,
                    source_pos.y + source_size.y / 2.0,
                );
                bends.add_values(
                    source_pos.x + source_size.x + node_bendpoint_padding,
                    bend_tmp,
                );
                bends.add_values(
                    target_pos.x - node_bendpoint_padding - edge_end_texture_padding,
                    bend_tmp,
                );
                bends.add_values(
                    target_pos.x - node_bendpoint_padding - edge_end_texture_padding,
                    target_pos.y + target_size.y / 2.0,
                );
                bends.add_values(target_pos.x, target_pos.y + target_size.y / 2.0);
            } else if direction == Direction::Up {
                let level_min = nd_level_min(nd, &source);
                bends.add_values(bend_tmp, level_min - node_bendpoint_padding);
                bends.add_values(
                    bend_tmp,
                    target_pos.y
                        + target_size.y
                        + node_bendpoint_padding
                        + edge_end_texture_padding,
                );
                bends.add_values(
                    target_pos.x + target_size.x / 2.0,
                    target_pos.y
                        + target_size.y
                        + node_bendpoint_padding
                        + edge_end_texture_padding,
                );
                bends.add_values(
                    target_pos.x + target_size.x / 2.0,
                    target_pos.y + target_size.y + node_bendpoint_padding,
                );
            } else {
                if !bends.is_empty() {
                    let mut last = bends.get_last();
                    last.y =
                        nd_level_max(nd, &source) + node_bendpoint_padding * *in_outs.second() as f64;
                    bends.set(bends.len() - 1, last);
                }
                bends.add_values(
                    bend_tmp,
                    nd_level_max(nd, &source) + node_bendpoint_padding * *in_outs.second() as f64,
                );
                bends.add_values(
                    bend_tmp,
                    target_pos.y
                        - node_bendpoint_padding * *in_outs.first() as f64
                        - edge_end_texture_padding,
                );
            }
        }

        Pair::of(side_one_edges, side_two_edges)
    }

    fn avoid_overlap_set_start_points(
        &self,
        graph: &TGraphRef,
        direction: Direction,
        node_bendpoint_padding: f64,
        nd: &HashMap<usize, NodeData>,
    ) {
        let nodes = graph
            .lock()
            .ok()
            .map(|g| g.nodes().clone())
            .unwrap_or_default();
        for node in nodes {
            if nd_is_super_root(nd, &node) {
                continue;
            }

            let mut outs = TreeUtil::get_all_outgoing_edges(&node, graph);
            if direction.is_horizontal() {
                outs.sort_by(|a, b| {
                    TreeUtil::get_first_point(a)
                        .y
                        .partial_cmp(&TreeUtil::get_first_point(b).y)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            } else {
                outs.sort_by(|a, b| {
                    TreeUtil::get_first_point(a)
                        .x
                        .partial_cmp(&TreeUtil::get_first_point(b).x)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }

            // Extract COMPACT_LEVEL_ASCENSION once per node (not per edge)
            let skip = node
                .lock()
                .ok()
                .and_then(|mut guard| {
                    guard.get_property(InternalProperties::COMPACT_LEVEL_ASCENSION)
                })
                .unwrap_or(false);
            let num = outs.len();
            for (i, edge) in outs.iter().enumerate() {
                if skip && !TreeUtil::is_cycle_inducing(edge, graph) {
                    continue;
                }

                let interpolation = if num == 1 {
                    Self::ONE_HALF
                } else {
                    (i + 1) as f64 / (num + 1) as f64
                };
                let pos = nd_pos(nd, &node);
                let size = nd_size(nd, &node);
                let level_min = nd_level_min(nd, &node);
                let level_max = nd_level_max(nd, &node);

                if let Ok(mut edge_guard) = edge.lock() {
                    let bends = edge_guard.bend_points();
                    if direction == Direction::Left {
                        let y = pos.y + size.y * interpolation;
                        let level_end = level_min;
                        bends.add_first_values(level_end.min(pos.x - node_bendpoint_padding), y);
                        bends.add_first_values(pos.x, y);
                    } else if direction == Direction::Right {
                        let y = pos.y + size.y * interpolation;
                        let level_end = level_max + node_bendpoint_padding;
                        bends.add_first_values(level_end, y);
                        bends.add_first_values(pos.x + size.x, y);
                    } else if direction == Direction::Up {
                        let x = pos.x + size.x * interpolation;
                        let level_end = level_min;
                        bends.add_first_values(x, (pos.y - node_bendpoint_padding).min(level_end));
                        bends.add_first_values(x, pos.y);
                    } else {
                        let x = pos.x + size.x * interpolation;
                        let level_end = level_max + node_bendpoint_padding;
                        bends.add_first_values(x, level_end);
                        bends.add_first_values(x, pos.y + size.y);
                    }
                }
            }
        }
    }

    fn avoid_overlap_set_end_points(
        &self,
        graph: &TGraphRef,
        direction: Direction,
        node_bendpoint_padding: f64,
        edge_end_texture_padding: f64,
        nd: &HashMap<usize, NodeData>,
    ) {
        let nodes = graph
            .lock()
            .ok()
            .map(|g| g.nodes().clone())
            .unwrap_or_default();
        for node in nodes {
            if nd_is_super_root(nd, &node) {
                continue;
            }

            let mut ins = TreeUtil::get_all_incoming_edges(&node, graph);
            if direction.is_horizontal() {
                ins.sort_by(|a, b| {
                    TreeUtil::get_last_point(a)
                        .y
                        .partial_cmp(&TreeUtil::get_last_point(b).y)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            } else {
                ins.sort_by(|a, b| {
                    TreeUtil::get_last_point(a)
                        .x
                        .partial_cmp(&TreeUtil::get_last_point(b).x)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }

            let num = ins.len();
            for (i, edge) in ins.iter().enumerate() {
                // Avoid locking the same edge recursively inside the routing branch.
                let is_cycle_inducing = TreeUtil::is_cycle_inducing(edge, graph);
                let interpolation = if num == 1 {
                    Self::ONE_HALF
                } else {
                    (i + 1) as f64 / (num + 1) as f64
                };

                let pos = nd_pos(nd, &node);
                let size = nd_size(nd, &node);
                let level_min = nd_level_min(nd, &node);
                let level_max = nd_level_max(nd, &node);

                if let Ok(mut edge_guard) = edge.lock() {
                    let bends = edge_guard.bend_points();
                    if direction == Direction::Left {
                        if pos.x + size.x + edge_end_texture_padding < level_max {
                            bends.add_values(
                                level_max + node_bendpoint_padding,
                                pos.y + size.y * interpolation,
                            );
                        } else if !bends.is_empty() {
                            let last = bends.get_last();
                            let next_x = pos.x + size.x / 2.0;
                            let next_y = pos.y + size.y / 2.0;
                            let denom =
                                (last.x - next_x).abs() / Self::STEEP_END_EDGE_SAMPLE_HEIGHT;
                            if edge_end_texture_padding > 0.0
                                && (last.y - next_y).abs() / denom
                                    > Self::STEEP_END_EDGE_THRESHOLD_DISTANCE
                            {
                                if next_y > last.y {
                                    bends.add_values(
                                        pos.x
                                            + size.x
                                            + edge_end_texture_padding / Self::STEEP_END_EDGE_RATIO,
                                        pos.y + size.y * interpolation
                                            - edge_end_texture_padding / 2.0,
                                    );
                                } else {
                                    bends.add_values(
                                        pos.x
                                            + size.x
                                            + edge_end_texture_padding / Self::STEEP_END_EDGE_RATIO,
                                        pos.y
                                            + size.y * interpolation
                                            + edge_end_texture_padding / 2.0,
                                    );
                                }
                            }
                        }
                        bends.add_values(pos.x + size.x, pos.y + size.y * interpolation);
                    } else if direction == Direction::Right {
                        if pos.x - edge_end_texture_padding > level_min {
                            bends.add_values(
                                level_min - node_bendpoint_padding,
                                pos.y + size.y * interpolation,
                            );
                        } else if !bends.is_empty() {
                            let last = bends.get_last();
                            let next_x = pos.x + size.x / 2.0;
                            let next_y = pos.y + size.y / 2.0;
                            let denom =
                                (last.x - next_x).abs() / Self::STEEP_END_EDGE_SAMPLE_HEIGHT;
                            if edge_end_texture_padding > 0.0
                                && (last.y - next_y).abs() / denom
                                    > Self::STEEP_END_EDGE_THRESHOLD_DISTANCE
                            {
                                if next_y > last.y {
                                    bends.add_values(
                                        pos.x
                                            - edge_end_texture_padding / Self::STEEP_END_EDGE_RATIO,
                                        pos.y + size.y * interpolation
                                            - edge_end_texture_padding / 2.0,
                                    );
                                } else {
                                    bends.add_values(
                                        pos.x
                                            - edge_end_texture_padding / Self::STEEP_END_EDGE_RATIO,
                                        pos.y
                                            + size.y * interpolation
                                            + edge_end_texture_padding / 2.0,
                                    );
                                }
                            }
                        }
                        bends.add_values(pos.x, pos.y + size.y * interpolation);
                    } else if direction == Direction::Up {
                        if pos.y + size.y + edge_end_texture_padding < level_max {
                            bends.add_values(
                                pos.x + size.x * interpolation,
                                level_max + node_bendpoint_padding,
                            );
                        } else if !bends.is_empty() {
                            let last = bends.get_last();
                            let next_x = pos.x + size.x / 2.0;
                            let next_y = pos.y + size.y / 2.0;
                            let denom =
                                (last.y - next_y).abs() / Self::STEEP_END_EDGE_SAMPLE_HEIGHT;
                            if edge_end_texture_padding > 0.0
                                && (last.x - next_x).abs() / denom
                                    > Self::STEEP_END_EDGE_THRESHOLD_DISTANCE
                            {
                                if next_x > last.x {
                                    bends.add_values(
                                        pos.x + size.x * interpolation
                                            - edge_end_texture_padding / 2.0,
                                        pos.y
                                            + edge_end_texture_padding / Self::STEEP_END_EDGE_RATIO
                                            + size.y,
                                    );
                                } else {
                                    bends.add_values(
                                        pos.x
                                            + size.x * interpolation
                                            + edge_end_texture_padding / 2.0,
                                        pos.y
                                            + edge_end_texture_padding / Self::STEEP_END_EDGE_RATIO
                                            + size.y,
                                    );
                                }
                            }
                        }
                        bends.add_values(pos.x + size.x * interpolation, pos.y + size.y);
                    } else {
                        if is_cycle_inducing {
                            if !bends.is_empty() {
                                bends
                                    .add_values(pos.x + size.x * interpolation, bends.get_last().y);
                            }
                        } else if pos.y - edge_end_texture_padding > level_min {
                            bends.add_values(
                                pos.x + size.x * interpolation,
                                level_min - node_bendpoint_padding,
                            );
                        } else if !bends.is_empty() {
                            let last = bends.get_last();
                            let next_x = pos.x + size.x / 2.0;
                            let next_y = pos.y + size.y / 2.0;
                            let denom =
                                (last.y - next_y).abs() / Self::STEEP_END_EDGE_SAMPLE_HEIGHT;
                            if edge_end_texture_padding > 0.0
                                && (last.x - next_x).abs() / denom
                                    > Self::STEEP_END_EDGE_THRESHOLD_DISTANCE
                            {
                                if next_x > last.x {
                                    bends.add_values(
                                        pos.x + size.x * interpolation
                                            - edge_end_texture_padding / 2.0,
                                        pos.y
                                            - edge_end_texture_padding / Self::STEEP_END_EDGE_RATIO,
                                    );
                                } else {
                                    bends.add_values(
                                        pos.x
                                            + size.x * interpolation
                                            + edge_end_texture_padding / 2.0,
                                        pos.y
                                            - edge_end_texture_padding / Self::STEEP_END_EDGE_RATIO,
                                    );
                                }
                            }
                        }
                        bends.add_values(pos.x + size.x * interpolation, pos.y);
                    }
                }
            }
        }
    }
}
