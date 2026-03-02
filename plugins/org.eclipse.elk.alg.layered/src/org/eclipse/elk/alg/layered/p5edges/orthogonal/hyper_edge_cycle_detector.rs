use std::collections::VecDeque;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;

use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment::HyperEdgeSegmentRef;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment_dependency::{
    DependencyType, HyperEdgeSegmentDependencyRef,
};

pub struct HyperEdgeCycleDetector;

impl HyperEdgeCycleDetector {
    pub fn detect_cycles(
        segments: &[HyperEdgeSegmentRef],
        critical_only: bool,
        random: &mut Random,
    ) -> Vec<HyperEdgeSegmentDependencyRef> {
        let mut result = Vec::new();
        let mut sources: VecDeque<HyperEdgeSegmentRef> = VecDeque::new();
        let mut sinks: VecDeque<HyperEdgeSegmentRef> = VecDeque::new();

        Self::initialize(segments, &mut sources, &mut sinks, critical_only);
        Self::compute_linear_ordering_marks(
            segments,
            &mut sources,
            &mut sinks,
            critical_only,
            random,
        );

        for source in segments {
            let outgoing = source.borrow().outgoing_segment_dependencies().clone();
            for dep in outgoing {
                let dep_guard = dep.borrow();
                if !critical_only || dep_guard.dependency_type() == DependencyType::Critical {
                    let target = dep_guard.target();
                    if let Some(target) = target {
                        if source.borrow().mark > target.borrow().mark {
                            result.push(dep.clone());
                        }
                    }
                }
            }
        }

        result
    }

    fn initialize(
        segments: &[HyperEdgeSegmentRef],
        sources: &mut VecDeque<HyperEdgeSegmentRef>,
        sinks: &mut VecDeque<HyperEdgeSegmentRef>,
        critical_only: bool,
    ) {
        let mut next_mark = -1;
        for segment in segments {
            let mut segment_guard = segment.borrow_mut();
            segment_guard.mark = next_mark;
            next_mark -= 1;

            let critical_in_weight: i32 = segment_guard
                .incoming_segment_dependencies()
                .iter()
                .filter(|dep| dep.borrow().dependency_type() == DependencyType::Critical)
                .map(|dep| dep.borrow().weight())
                .sum();

            let critical_out_weight: i32 = segment_guard
                .outgoing_segment_dependencies()
                .iter()
                .filter(|dep| dep.borrow().dependency_type() == DependencyType::Critical)
                .map(|dep| dep.borrow().weight())
                .sum();

            let mut in_weight = critical_in_weight;
            let mut out_weight = critical_out_weight;

            if !critical_only {
                in_weight = segment_guard
                    .incoming_segment_dependencies()
                    .iter()
                    .map(|dep| dep.borrow().weight())
                    .sum();
                out_weight = segment_guard
                    .outgoing_segment_dependencies()
                    .iter()
                    .map(|dep| dep.borrow().weight())
                    .sum();
            }

            segment_guard.set_in_weight(in_weight);
            segment_guard.set_critical_in_weight(critical_in_weight);
            segment_guard.set_out_weight(out_weight);
            segment_guard.set_critical_out_weight(critical_out_weight);

            if out_weight == 0 {
                sinks.push_back(segment.clone());
            } else if in_weight == 0 {
                sources.push_back(segment.clone());
            }
        }
    }

    fn compute_linear_ordering_marks(
        segments: &[HyperEdgeSegmentRef],
        sources: &mut VecDeque<HyperEdgeSegmentRef>,
        sinks: &mut VecDeque<HyperEdgeSegmentRef>,
        critical_only: bool,
        random: &mut Random,
    ) {
        let mut unprocessed: Vec<HyperEdgeSegmentRef> = segments.to_vec();
        unprocessed.sort_by(|a, b| a.borrow().mark.cmp(&b.borrow().mark));
        let mut max_segments: Vec<HyperEdgeSegmentRef> = Vec::new();

        let mark_base = segments.len() as i32;
        let mut next_sink_mark = mark_base - 1;
        let mut next_source_mark = mark_base + 1;

        while !unprocessed.is_empty() {
            while let Some(sink) = sinks.pop_front() {
                remove_segment(&mut unprocessed, &sink);
                sink.borrow_mut().mark = next_sink_mark;
                next_sink_mark -= 1;
                Self::update_neighbors(&sink, sources, sinks, critical_only);
            }

            while let Some(source) = sources.pop_front() {
                remove_segment(&mut unprocessed, &source);
                source.borrow_mut().mark = next_source_mark;
                next_source_mark += 1;
                Self::update_neighbors(&source, sources, sinks, critical_only);
            }

            let mut max_outflow = i32::MIN;
            max_segments.clear();

            for segment in &unprocessed {
                let segment_guard = segment.borrow();
                if !critical_only
                    && segment_guard.critical_out_weight() > 0
                    && segment_guard.critical_in_weight() <= 0
                {
                    max_segments.clear();
                    max_segments.push(segment.clone());
                    break;
                }

                let outflow = segment_guard.out_weight() - segment_guard.in_weight();
                if outflow >= max_outflow {
                    if outflow > max_outflow {
                        max_segments.clear();
                        max_outflow = outflow;
                    }
                    max_segments.push(segment.clone());
                }
            }

            if !max_segments.is_empty() {
                let pick_index = random.next_int(max_segments.len() as i32) as usize;
                let max_node = max_segments[pick_index].clone();
                remove_segment(&mut unprocessed, &max_node);
                max_node.borrow_mut().mark = next_source_mark;
                next_source_mark += 1;
                Self::update_neighbors(&max_node, sources, sinks, critical_only);
            }
        }

        let shift_base = segments.len() as i32 + 1;
        for segment in segments {
            let mut seg_guard = segment.borrow_mut();
            if seg_guard.mark < mark_base {
                seg_guard.mark += shift_base;
            }
        }
    }

    fn update_neighbors(
        node: &HyperEdgeSegmentRef,
        sources: &mut VecDeque<HyperEdgeSegmentRef>,
        sinks: &mut VecDeque<HyperEdgeSegmentRef>,
        critical_only: bool,
    ) {
        let outgoing = node.borrow().outgoing_segment_dependencies().clone();
        for dep in outgoing {
            let dep_guard = dep.borrow();
            if !critical_only || dep_guard.dependency_type() == DependencyType::Critical {
                if let Some(target) = dep_guard.target() {
                    let mut tgt = target.borrow_mut();
                    if tgt.mark < 0 && dep_guard.weight() > 0 {
                        let new_in_weight = tgt.in_weight() - dep_guard.weight();
                        tgt.set_in_weight(new_in_weight);
                        if dep_guard.dependency_type() == DependencyType::Critical {
                            let new_weight = tgt.critical_in_weight() - dep_guard.weight();
                            tgt.set_critical_in_weight(new_weight);
                        }
                        let out_weight = tgt.out_weight();
                        drop(tgt);
                        if new_in_weight <= 0 && out_weight > 0 {
                            sources.push_back(target.clone());
                        }
                    }
                }
            }
        }

        let incoming = node.borrow().incoming_segment_dependencies().clone();
        for dep in incoming {
            let dep_guard = dep.borrow();
            if !critical_only || dep_guard.dependency_type() == DependencyType::Critical {
                if let Some(source) = dep_guard.source() {
                    let mut src = source.borrow_mut();
                    if src.mark < 0 && dep_guard.weight() > 0 {
                        let new_out_weight = src.out_weight() - dep_guard.weight();
                        src.set_out_weight(new_out_weight);
                        if dep_guard.dependency_type() == DependencyType::Critical {
                            let new_weight = src.critical_out_weight() - dep_guard.weight();
                            src.set_critical_out_weight(new_weight);
                        }
                        let in_weight = src.in_weight();
                        drop(src);
                        if new_out_weight <= 0 && in_weight > 0 {
                            sinks.push_back(source.clone());
                        }
                    }
                }
            }
        }
    }
}

fn remove_segment(list: &mut Vec<HyperEdgeSegmentRef>, segment: &HyperEdgeSegmentRef) {
    if let Some(pos) = list.iter().position(|item| Rc::ptr_eq(item, segment)) {
        list.remove(pos);
    }
}
