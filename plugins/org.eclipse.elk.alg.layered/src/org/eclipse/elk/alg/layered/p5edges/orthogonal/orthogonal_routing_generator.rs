use std::collections::VecDeque;

use rustc_hash::FxHashMap;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{IElkProgressMonitor, Random};
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNodeRef};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, PortType};
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::direction::base_routing_direction_strategy::RoutingDirectionStrategy;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::direction::routing_direction::RoutingDirection;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_cycle_detector::HyperEdgeCycleDetector;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment::{HyperEdgeSegment, HyperEdgeSegmentRef};
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment_dependency::HyperEdgeSegmentDependency;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment_splitter::HyperEdgeSegmentSplitter;

pub struct OrthogonalRoutingGenerator {
    routing_direction: RoutingDirection,
    routing_strategy: RoutingDirectionStrategy,
    edge_spacing: f64,
    conflict_threshold: f64,
    critical_conflict_threshold: f64,
    debug_prefix: Option<String>,
}

impl OrthogonalRoutingGenerator {
    pub const TOLERANCE: f64 = 1e-3;

    const CRITICAL_CONFLICTS_DETECTED: i32 = -1;
    const CONFLICT_THRESHOLD_FACTOR: f64 = 0.5;
    const CRITICAL_CONFLICT_THRESHOLD_FACTOR: f64 = 0.2;

    const CONFLICT_PENALTY: i32 = 1;
    const CROSSING_PENALTY: i32 = 16;

    pub fn new(
        direction: RoutingDirection,
        edge_spacing: f64,
        debug_prefix: impl Into<Option<String>>,
    ) -> Self {
        let debug_prefix = debug_prefix.into();
        OrthogonalRoutingGenerator {
            routing_direction: direction,
            routing_strategy: RoutingDirectionStrategy::for_routing_direction(direction),
            edge_spacing,
            conflict_threshold: Self::CONFLICT_THRESHOLD_FACTOR * edge_spacing,
            critical_conflict_threshold: 0.0,
            debug_prefix,
        }
    }

    pub fn route_edges(
        &mut self,
        _monitor: &mut dyn IElkProgressMonitor,
        layered_graph: &mut LGraph,
        source_layer_nodes: Option<&[LNodeRef]>,
        _source_layer_index: i32,
        target_layer_nodes: Option<&[LNodeRef]>,
        start_pos: f64,
    ) -> i32 {
        let mut port_to_segment_map: FxHashMap<usize, HyperEdgeSegmentRef> = FxHashMap::default();
        let mut edge_segments: Vec<HyperEdgeSegmentRef> = Vec::new();

        self.create_hyper_edge_segments(
            source_layer_nodes,
            self.routing_strategy.get_source_port_side(),
            &mut edge_segments,
            &mut port_to_segment_map,
        );
        self.create_hyper_edge_segments(
            target_layer_nodes,
            self.routing_strategy.get_target_port_side(),
            &mut edge_segments,
            &mut port_to_segment_map,
        );
        self.trace_segments("initial", &edge_segments);

        self.critical_conflict_threshold = Self::CRITICAL_CONFLICT_THRESHOLD_FACTOR
            * self.minimum_horizontal_segment_distance(&edge_segments);

        let mut critical_dependency_count = 0;
        for i in 0..edge_segments.len() {
            for j in (i + 1)..edge_segments.len() {
                let first = &edge_segments[i];
                let second = &edge_segments[j];
                critical_dependency_count += self.create_dependency_if_necessary(first, second);
            }
        }

        let _ = &self.debug_prefix;
        let random = layered_graph
            .get_property(InternalProperties::RANDOM)
            .unwrap_or_default();
        let mut random = random;
        if critical_dependency_count >= 2 {
            self.break_critical_cycles(&mut edge_segments, &mut random);
        }
        self.trace_segments("after_critical", &edge_segments);

        Self::break_non_critical_cycles(&edge_segments, &mut random);
        self.trace_segments("after_noncritical", &edge_segments);

        Self::topological_numbering(&edge_segments);

        let mut rank_count = -1;
        for segment in &edge_segments {
            let segment_guard = segment.borrow();
            if (segment_guard.start_coordinate() - segment_guard.end_coordinate()).abs()
                < Self::TOLERANCE
            {
                continue;
            }
            rank_count = rank_count.max(segment_guard.routing_slot());
            self.routing_strategy.calculate_bend_points(
                &segment_guard,
                start_pos,
                self.edge_spacing,
            );
        }

        self.routing_strategy.clear_created_junction_points();
        rank_count + 1
    }

    fn trace_segments(&self, label: &str, edge_segments: &[HyperEdgeSegmentRef]) {
        if !ElkTrace::global().ortho {
            return;
        }

        let prefix = self.debug_prefix.as_deref().unwrap_or("?");
        eprintln!(
            "[orthogonal:{}] {} segments={}",
            prefix,
            label,
            edge_segments.len()
        );

        for (idx, segment) in edge_segments.iter().enumerate() {
            let seg = segment.borrow();
            let ports = seg
                .ports()
                .iter()
                .map(|port| {
                    if let Some(port_guard) = port.lock_ok() {
                        let node_name = port_guard
                            .node()
                            .and_then(|node| {
                                node.lock_ok()
                                    .map(|mut node_guard| node_guard.designation())
                            })
                            .unwrap_or_else(|| "?".to_string());
                        let anchor = port_guard.absolute_anchor().map(|a| a.y).unwrap_or(0.0);
                        format!("{}:{:?}@{:.1}", node_name, port_guard.side(), anchor)
                    } else {
                        "?".to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");

            eprintln!(
                "  seg#{idx} slot={} start={:.1} end={:.1} incoming={:?} outgoing={:?} split_by={} split_partner={} ports=[{}]",
                seg.routing_slot(),
                seg.start_coordinate(),
                seg.end_coordinate(),
                seg.incoming_connection_coordinates(),
                seg.outgoing_connection_coordinates(),
                seg.split_by().is_some(),
                seg.split_partner().is_some(),
                ports
            );
        }
    }

    fn create_hyper_edge_segments(
        &self,
        nodes: Option<&[LNodeRef]>,
        port_side: PortSide,
        hyper_edges: &mut Vec<HyperEdgeSegmentRef>,
        port_map: &mut FxHashMap<usize, HyperEdgeSegmentRef>,
    ) {
        if let Some(nodes) = nodes {
            for node in nodes {
                let ports = node
                    .lock_ok()
                    .map(|node_guard| {
                        node_guard.ports_by_type_and_side(PortType::Output, port_side)
                    })
                    .unwrap_or_default();
                for port in ports {
                    let key = port_key(&port);
                    let entry = port_map.get(&key).cloned();
                    let segment = if let Some(existing) = entry {
                        existing
                    } else {
                        let segment = HyperEdgeSegment::new(self.routing_direction);
                        hyper_edges.push(segment.clone());
                        segment
                    };
                    if !port_map.contains_key(&key) {
                        HyperEdgeSegment::add_port_positions(&segment, &port, port_map);
                    }
                }
            }
        }
    }

    pub fn create_dependency_if_necessary(
        &self,
        he1: &HyperEdgeSegmentRef,
        he2: &HyperEdgeSegmentRef,
    ) -> i32 {
        // Borrow both segments simultaneously (different Rc's → different RefCells).
        // Compute all scalar results while borrows are alive to avoid 4x Vec<f64> clone.
        let (conflicts1, conflicts2, dep_value1, dep_value2) = {
            let he1_guard = he1.borrow();
            let he2_guard = he2.borrow();

            let he1_start = he1_guard.start_coordinate();
            let he1_end = he1_guard.end_coordinate();
            let he2_start = he2_guard.start_coordinate();
            let he2_end = he2_guard.end_coordinate();

            if (he1_start - he1_end).abs() < Self::TOLERANCE
                || (he2_start - he2_end).abs() < Self::TOLERANCE
            {
                return 0;
            }

            let he1_outgoing = he1_guard.outgoing_connection_coordinates();
            let he1_incoming = he1_guard.incoming_connection_coordinates();
            let he2_outgoing = he2_guard.outgoing_connection_coordinates();
            let he2_incoming = he2_guard.incoming_connection_coordinates();

            let conflicts1 = self.count_conflicts(he1_outgoing, he2_incoming);
            let conflicts2 = self.count_conflicts(he2_outgoing, he1_incoming);

            let critical_conflicts_detected = conflicts1 == Self::CRITICAL_CONFLICTS_DETECTED
                || conflicts2 == Self::CRITICAL_CONFLICTS_DETECTED;

            let (dep_value1, dep_value2) = if critical_conflicts_detected {
                (0, 0)
            } else {
                let crossings1 = Self::count_crossings(he1_outgoing, he2_start, he2_end)
                    + Self::count_crossings(he2_incoming, he1_start, he1_end);
                let crossings2 = Self::count_crossings(he2_outgoing, he1_start, he1_end)
                    + Self::count_crossings(he1_incoming, he2_start, he2_end);
                (
                    Self::CONFLICT_PENALTY * conflicts1 + Self::CROSSING_PENALTY * crossings1,
                    Self::CONFLICT_PENALTY * conflicts2 + Self::CROSSING_PENALTY * crossings2,
                )
            };

            (conflicts1, conflicts2, dep_value1, dep_value2)
        };

        let critical_conflicts_detected = conflicts1 == Self::CRITICAL_CONFLICTS_DETECTED
            || conflicts2 == Self::CRITICAL_CONFLICTS_DETECTED;
        let mut critical_dependency_count = 0;

        if critical_conflicts_detected {
            if conflicts1 == Self::CRITICAL_CONFLICTS_DETECTED {
                HyperEdgeSegmentDependency::create_and_add_critical(he2, he1);
                critical_dependency_count += 1;
            }
            if conflicts2 == Self::CRITICAL_CONFLICTS_DETECTED {
                HyperEdgeSegmentDependency::create_and_add_critical(he1, he2);
                critical_dependency_count += 1;
            }
        } else if dep_value1 < dep_value2 {
            HyperEdgeSegmentDependency::create_and_add_regular(
                he1,
                he2,
                dep_value2 - dep_value1,
            );
        } else if dep_value1 > dep_value2 {
            HyperEdgeSegmentDependency::create_and_add_regular(
                he2,
                he1,
                dep_value1 - dep_value2,
            );
        } else if dep_value1 > 0 && dep_value2 > 0 {
            HyperEdgeSegmentDependency::create_and_add_regular(he1, he2, 0);
            HyperEdgeSegmentDependency::create_and_add_regular(he2, he1, 0);
        }

        critical_dependency_count
    }

    fn count_conflicts(&self, posis1: &[f64], posis2: &[f64]) -> i32 {
        let mut conflicts = 0;
        if !posis1.is_empty() && !posis2.is_empty() {
            // Keep Java parity: when one iterator is exhausted on equal values, the other
            // iterator may still advance. This affects critical conflict detection.
            let mut i = 0usize;
            let mut j = 0usize;
            let mut pos1 = posis1[i];
            let mut pos2 = posis2[j];
            let mut has_more = true;
            while has_more {
                if pos1 > pos2 - self.critical_conflict_threshold
                    && pos1 < pos2 + self.critical_conflict_threshold
                {
                    return Self::CRITICAL_CONFLICTS_DETECTED;
                } else if pos1 > pos2 - self.conflict_threshold
                    && pos1 < pos2 + self.conflict_threshold
                {
                    conflicts += 1;
                }

                if pos1 <= pos2 && i + 1 < posis1.len() {
                    i += 1;
                    pos1 = posis1[i];
                } else if pos2 <= pos1 && j + 1 < posis2.len() {
                    j += 1;
                    pos2 = posis2[j];
                } else {
                    has_more = false;
                }
            }
        }

        conflicts
    }

    pub fn count_crossings(posis: &[f64], start: f64, end: f64) -> i32 {
        let mut crossings = 0;
        for pos in posis {
            if *pos > end {
                break;
            } else if *pos >= start {
                crossings += 1;
            }
        }
        crossings
    }

    fn minimum_horizontal_segment_distance(&self, edge_segments: &[HyperEdgeSegmentRef]) -> f64 {
        let mut incoming: Vec<f64> = Vec::new();
        let mut outgoing: Vec<f64> = Vec::new();
        for segment in edge_segments {
            let segment_guard = segment.borrow();
            incoming.extend(
                segment_guard
                    .incoming_connection_coordinates()
                    .iter()
                    .cloned(),
            );
            outgoing.extend(
                segment_guard
                    .outgoing_connection_coordinates()
                    .iter()
                    .cloned(),
            );
        }

        let min_incoming = Self::minimum_difference(&incoming);
        let min_outgoing = Self::minimum_difference(&outgoing);

        min_incoming.min(min_outgoing)
    }

    fn minimum_difference(numbers: &[f64]) -> f64 {
        if numbers.len() < 2 {
            return f64::MAX;
        }
        let mut sorted = numbers.to_vec();
        // Java Stream.sorted() uses Double.compare which defines a total order (incl. NaN/-0.0).
        sorted.sort_by(|a, b| a.total_cmp(b));
        // Match Double#equals semantics (NaN == NaN, -0.0 != 0.0).
        sorted.dedup_by(|a, b| a.to_bits() == b.to_bits());

        if sorted.len() < 2 {
            return f64::MAX;
        }

        let mut min_difference = f64::MAX;
        for window in sorted.windows(2) {
            let diff = window[1] - window[0];
            if diff.is_nan() {
                return f64::NAN;
            }
            if diff < min_difference {
                min_difference = diff;
            }
        }
        min_difference
    }

    fn break_critical_cycles(
        &mut self,
        edge_segments: &mut Vec<HyperEdgeSegmentRef>,
        random: &mut Random,
    ) {
        let cycle_dependencies = HyperEdgeCycleDetector::detect_cycles(edge_segments, true, random);
        HyperEdgeSegmentSplitter::split_segments(
            self,
            &cycle_dependencies,
            edge_segments,
            self.critical_conflict_threshold,
        );
    }

    pub fn break_non_critical_cycles(edge_segments: &[HyperEdgeSegmentRef], random: &mut Random) {
        let cycle_dependencies =
            HyperEdgeCycleDetector::detect_cycles(edge_segments, false, random);
        for dep in cycle_dependencies {
            if dep.borrow().weight() == 0 {
                HyperEdgeSegmentDependency::remove(&dep);
            } else {
                HyperEdgeSegmentDependency::reverse(&dep);
            }
        }
    }

    fn topological_numbering(segments: &[HyperEdgeSegmentRef]) {
        let mut sources: VecDeque<HyperEdgeSegmentRef> = VecDeque::new();
        let mut rightward_targets: VecDeque<HyperEdgeSegmentRef> = VecDeque::new();
        for segment in segments {
            // Single borrow_mut to read lengths and set weights
            let mut segment_guard = segment.borrow_mut();
            let in_weight = segment_guard.incoming_segment_dependencies().len() as i32;
            let out_weight = segment_guard.outgoing_segment_dependencies().len() as i32;
            segment_guard.set_in_weight(in_weight);
            segment_guard.set_out_weight(out_weight);
            let no_incoming_coords = segment_guard.incoming_connection_coordinates().is_empty();
            drop(segment_guard);
            if in_weight == 0 {
                sources.push_back(segment.clone());
            }
            if out_weight == 0 && no_incoming_coords {
                rightward_targets.push_back(segment.clone());
            }
        }

        let mut max_rank = -1;
        while let Some(node) = sources.pop_front() {
            // Keep node borrow alive to avoid Vec clone of outgoing deps
            let node_guard = node.borrow();
            let node_slot = node_guard.routing_slot();
            for dep in node_guard.outgoing_segment_dependencies() {
                if let Some(target) = dep.borrow().target() {
                    let new_slot = node_slot + 1;
                    let mut tgt = target.borrow_mut();
                    if tgt.routing_slot() < new_slot {
                        tgt.set_routing_slot(new_slot);
                    }
                    max_rank = max_rank.max(tgt.routing_slot());
                    let in_weight = tgt.in_weight() - 1;
                    tgt.set_in_weight(in_weight);
                    drop(tgt);
                    if in_weight == 0 {
                        sources.push_back(target.clone());
                    }
                }
            }
        }

        if max_rank > -1 {
            for node in &rightward_targets {
                node.borrow_mut().set_routing_slot(max_rank);
            }

            while let Some(node) = rightward_targets.pop_front() {
                // Keep node borrow alive to avoid Vec clone of incoming deps
                let node_guard = node.borrow();
                let node_slot = node_guard.routing_slot();
                for dep in node_guard.incoming_segment_dependencies() {
                    if let Some(source) = dep.borrow().source() {
                        if !source.borrow().incoming_connection_coordinates().is_empty() {
                            continue;
                        }
                        let new_slot = node_slot - 1;
                        let mut src = source.borrow_mut();
                        if src.routing_slot() > new_slot {
                            src.set_routing_slot(new_slot);
                        }
                        let out_weight = src.out_weight() - 1;
                        src.set_out_weight(out_weight);
                        drop(src);
                        if out_weight == 0 {
                            rightward_targets.push_back(source.clone());
                        }
                    }
                }
            }
        }
    }
}

fn port_key(port: &crate::org::eclipse::elk::alg::layered::graph::LPortRef) -> usize {
    std::sync::Arc::as_ptr(port) as usize
}

#[cfg(test)]
mod tests {
    use super::OrthogonalRoutingGenerator;

    #[test]
    fn minimum_difference_handles_nan_like_java() {
        let values = vec![1.0, f64::NAN, 3.0, f64::NAN];
        let result = OrthogonalRoutingGenerator::minimum_difference(&values);
        assert!(
            result.is_nan(),
            "expected NaN minimum difference when NaN is present"
        );
    }
}
