use std::collections::HashMap;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{IElkProgressMonitor, Random};

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

    pub fn new(direction: RoutingDirection, edge_spacing: f64, debug_prefix: impl Into<Option<String>>) -> Self {
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
        let mut port_to_segment_map: HashMap<usize, HyperEdgeSegmentRef> = HashMap::new();
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

        self.critical_conflict_threshold =
            Self::CRITICAL_CONFLICT_THRESHOLD_FACTOR * self.minimum_horizontal_segment_distance(&edge_segments);

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

        Self::break_non_critical_cycles(&edge_segments, &mut random);

        Self::topological_numbering(&edge_segments);

        let mut rank_count = -1;
        for segment in &edge_segments {
            let segment_guard = segment.borrow();
            if (segment_guard.start_coordinate() - segment_guard.end_coordinate()).abs() < Self::TOLERANCE {
                continue;
            }
            rank_count = rank_count.max(segment_guard.routing_slot());
            self.routing_strategy
                .calculate_bend_points(&segment_guard, start_pos, self.edge_spacing);
        }

        self.routing_strategy.clear_created_junction_points();
        rank_count + 1
    }

    fn create_hyper_edge_segments(
        &self,
        nodes: Option<&[LNodeRef]>,
        port_side: PortSide,
        hyper_edges: &mut Vec<HyperEdgeSegmentRef>,
        port_map: &mut HashMap<usize, HyperEdgeSegmentRef>,
    ) {
        if let Some(nodes) = nodes {
            for node in nodes {
                let ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports_by_type_and_side(PortType::Output, port_side))
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
        let (he1_start, he1_end, he1_outgoing, he1_incoming) = {
            let he1_guard = he1.borrow();
            (
                he1_guard.start_coordinate(),
                he1_guard.end_coordinate(),
                he1_guard.outgoing_connection_coordinates().clone(),
                he1_guard.incoming_connection_coordinates().clone(),
            )
        };
        let (he2_start, he2_end, he2_outgoing, he2_incoming) = {
            let he2_guard = he2.borrow();
            (
                he2_guard.start_coordinate(),
                he2_guard.end_coordinate(),
                he2_guard.outgoing_connection_coordinates().clone(),
                he2_guard.incoming_connection_coordinates().clone(),
            )
        };

        if (he1_start - he1_end).abs() < Self::TOLERANCE
            || (he2_start - he2_end).abs() < Self::TOLERANCE
        {
            return 0;
        }

        let conflicts1 = self.count_conflicts(&he1_outgoing, &he2_incoming);
        let conflicts2 = self.count_conflicts(&he2_outgoing, &he1_incoming);

        let critical_conflicts_detected =
            conflicts1 == Self::CRITICAL_CONFLICTS_DETECTED || conflicts2 == Self::CRITICAL_CONFLICTS_DETECTED;
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
        } else {
            let crossings1 =
                Self::count_crossings(&he1_outgoing, he2_start, he2_end)
                    + Self::count_crossings(&he2_incoming, he1_start, he1_end);
            let crossings2 =
                Self::count_crossings(&he2_outgoing, he1_start, he1_end)
                    + Self::count_crossings(&he1_incoming, he2_start, he2_end);

            let dep_value1 = Self::CONFLICT_PENALTY * conflicts1 + Self::CROSSING_PENALTY * crossings1;
            let dep_value2 = Self::CONFLICT_PENALTY * conflicts2 + Self::CROSSING_PENALTY * crossings2;

            if dep_value1 < dep_value2 {
                HyperEdgeSegmentDependency::create_and_add_regular(he1, he2, dep_value2 - dep_value1);
            } else if dep_value1 > dep_value2 {
                HyperEdgeSegmentDependency::create_and_add_regular(he2, he1, dep_value1 - dep_value2);
            } else if dep_value1 > 0 && dep_value2 > 0 {
                HyperEdgeSegmentDependency::create_and_add_regular(he1, he2, 0);
                HyperEdgeSegmentDependency::create_and_add_regular(he2, he1, 0);
            }
        }

        critical_dependency_count
    }

    fn count_conflicts(&self, posis1: &[f64], posis2: &[f64]) -> i32 {
        let mut conflicts = 0;
        if !posis1.is_empty() && !posis2.is_empty() {
            let mut i = 0;
            let mut j = 0;
            while i < posis1.len() && j < posis2.len() {
                let pos1 = posis1[i];
                let pos2 = posis2[j];

                if pos1 > pos2 - self.critical_conflict_threshold
                    && pos1 < pos2 + self.critical_conflict_threshold
                {
                    return Self::CRITICAL_CONFLICTS_DETECTED;
                } else if pos1 > pos2 - self.conflict_threshold && pos1 < pos2 + self.conflict_threshold {
                    conflicts += 1;
                }

                if pos1 <= pos2 {
                    i += 1;
                } else {
                    j += 1;
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
            incoming.extend(segment_guard.incoming_connection_coordinates().iter().cloned());
            outgoing.extend(segment_guard.outgoing_connection_coordinates().iter().cloned());
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

    fn break_critical_cycles(&mut self, edge_segments: &mut Vec<HyperEdgeSegmentRef>, random: &mut Random) {
        let cycle_dependencies = HyperEdgeCycleDetector::detect_cycles(edge_segments, true, random);
        HyperEdgeSegmentSplitter::split_segments(
            self,
            &cycle_dependencies,
            edge_segments,
            self.critical_conflict_threshold,
        );
    }

    pub fn break_non_critical_cycles(edge_segments: &[HyperEdgeSegmentRef], random: &mut Random) {
        let cycle_dependencies = HyperEdgeCycleDetector::detect_cycles(edge_segments, false, random);
        for dep in cycle_dependencies {
            if dep.borrow().weight() == 0 {
                HyperEdgeSegmentDependency::remove(&dep);
            } else {
                HyperEdgeSegmentDependency::reverse(&dep);
            }
        }
    }

    fn topological_numbering(segments: &[HyperEdgeSegmentRef]) {
        let mut sources: Vec<HyperEdgeSegmentRef> = Vec::new();
        let mut rightward_targets: Vec<HyperEdgeSegmentRef> = Vec::new();
        for segment in segments {
            let in_weight = segment.borrow().incoming_segment_dependencies().len() as i32;
            let out_weight = segment.borrow().outgoing_segment_dependencies().len() as i32;
            {
                let mut segment_guard = segment.borrow_mut();
                segment_guard.set_in_weight(in_weight);
                segment_guard.set_out_weight(out_weight);
            }
            if in_weight == 0 {
                sources.push(segment.clone());
            }
            if out_weight == 0 && segment.borrow().incoming_connection_coordinates().is_empty() {
                rightward_targets.push(segment.clone());
            }
        }

        let mut max_rank = -1;
        while let Some(node) = pop_front(&mut sources) {
            let outgoing = node.borrow().outgoing_segment_dependencies().clone();
            for dep in outgoing {
                let target = dep.borrow().target();
                if let Some(target) = target {
                    let new_slot = node.borrow().routing_slot() + 1;
                    if target.borrow().routing_slot() < new_slot {
                        target.borrow_mut().set_routing_slot(new_slot);
                    }
                    max_rank = max_rank.max(target.borrow().routing_slot());
                    let in_weight = target.borrow().in_weight() - 1;
                    target.borrow_mut().set_in_weight(in_weight);
                    if in_weight == 0 {
                        sources.push(target.clone());
                    }
                }
            }
        }

        if max_rank > -1 {
            for node in &rightward_targets {
                node.borrow_mut().set_routing_slot(max_rank);
            }

            while let Some(node) = pop_front(&mut rightward_targets) {
                let incoming = node.borrow().incoming_segment_dependencies().clone();
                for dep in incoming {
                    let source = dep.borrow().source();
                    if let Some(source) = source {
                        if !source.borrow().incoming_connection_coordinates().is_empty() {
                            continue;
                        }
                        let new_slot = node.borrow().routing_slot() - 1;
                        if source.borrow().routing_slot() > new_slot {
                            source.borrow_mut().set_routing_slot(new_slot);
                        }
                        let out_weight = source.borrow().out_weight() - 1;
                        source.borrow_mut().set_out_weight(out_weight);
                        if out_weight == 0 {
                            rightward_targets.push(source.clone());
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

fn pop_front(list: &mut Vec<HyperEdgeSegmentRef>) -> Option<HyperEdgeSegmentRef> {
    if list.is_empty() {
        None
    } else {
        Some(list.remove(0))
    }
}

#[cfg(test)]
mod tests {
    use super::OrthogonalRoutingGenerator;

    #[test]
    fn minimum_difference_handles_nan_like_java() {
        let values = vec![1.0, f64::NAN, 3.0, f64::NAN];
        let result = OrthogonalRoutingGenerator::minimum_difference(&values);
        assert!(result.is_nan(), "expected NaN minimum difference when NaN is present");
    }
}
