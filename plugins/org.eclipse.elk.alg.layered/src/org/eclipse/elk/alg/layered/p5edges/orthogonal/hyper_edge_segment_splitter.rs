use std::cmp::Ordering;
use std::rc::Rc;

use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment::{HyperEdgeSegment, HyperEdgeSegmentRef};
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment_dependency::HyperEdgeSegmentDependency;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment_dependency::HyperEdgeSegmentDependencyRef;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::orthogonal_routing_generator::OrthogonalRoutingGenerator;

pub struct HyperEdgeSegmentSplitter;

impl HyperEdgeSegmentSplitter {
    pub fn split_segments(
        routing_generator: &mut OrthogonalRoutingGenerator,
        dependencies_to_resolve: &[HyperEdgeSegmentDependencyRef],
        segments: &mut Vec<HyperEdgeSegmentRef>,
        critical_conflict_threshold: f64,
    ) {
        if dependencies_to_resolve.is_empty() {
            return;
        }

        let mut free_areas = Self::find_free_areas(segments, critical_conflict_threshold);
        let mut segments_to_split = Self::decide_which_segments_to_split(dependencies_to_resolve);

        segments_to_split.sort_by(|a, b| {
            let len_a = a.borrow().length();
            let len_b = b.borrow().length();
            len_a
                .partial_cmp(&len_b)
                .unwrap_or(Ordering::Equal)
        });

        for segment in segments_to_split {
            Self::split(
                routing_generator,
                &segment,
                segments,
                &mut free_areas,
                critical_conflict_threshold,
            );
        }
    }

    fn find_free_areas(
        segments: &[HyperEdgeSegmentRef],
        critical_conflict_threshold: f64,
    ) -> Vec<FreeArea> {
        let mut coordinates: Vec<f64> = Vec::new();
        for segment in segments {
            let segment_guard = segment.borrow();
            coordinates.extend(segment_guard.incoming_connection_coordinates().iter().cloned());
            coordinates.extend(segment_guard.outgoing_connection_coordinates().iter().cloned());
        }
        coordinates.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut free_areas = Vec::new();
        for i in 1..coordinates.len() {
            if coordinates[i] - coordinates[i - 1] >= 2.0 * critical_conflict_threshold {
                free_areas.push(FreeArea::new(
                    coordinates[i - 1] + critical_conflict_threshold,
                    coordinates[i] - critical_conflict_threshold,
                ));
            }
        }
        free_areas
    }

    fn decide_which_segments_to_split(
        dependencies: &[HyperEdgeSegmentDependencyRef],
    ) -> Vec<HyperEdgeSegmentRef> {
        let mut segments_to_split: Vec<HyperEdgeSegmentRef> = Vec::new();
        for dep in dependencies {
            let dep_guard = dep.borrow();
            let source = dep_guard.source();
            let target = dep_guard.target();
            let (Some(source), Some(target)) = (source, target) else { continue };

            if contains_segment(&segments_to_split, &source) || contains_segment(&segments_to_split, &target) {
                continue;
            }

            let mut segment_to_split = source.clone();
            let mut segment_causing_split = target.clone();

            if source.borrow().represents_hyperedge() && !target.borrow().represents_hyperedge() {
                segment_to_split = target.clone();
                segment_causing_split = source.clone();
            }

            segment_to_split
                .borrow_mut()
                .set_split_by(Some(&segment_causing_split));
            segments_to_split.push(segment_to_split);
        }

        segments_to_split
    }

    fn split(
        routing_generator: &mut OrthogonalRoutingGenerator,
        segment: &HyperEdgeSegmentRef,
        segments: &mut Vec<HyperEdgeSegmentRef>,
        free_areas: &mut Vec<FreeArea>,
        critical_conflict_threshold: f64,
    ) {
        let split_position =
            Self::compute_position_to_split_and_update_free_areas(segment, free_areas, critical_conflict_threshold);
        let new_segment = HyperEdgeSegment::split_at(segment, split_position);
        segments.push(new_segment);
        Self::update_dependencies(routing_generator, segment, segments);
    }

    fn update_dependencies(
        routing_generator: &mut OrthogonalRoutingGenerator,
        segment: &HyperEdgeSegmentRef,
        segments: &[HyperEdgeSegmentRef],
    ) {
        let split_causing = segment.borrow().split_by();
        let split_partner = segment.borrow().split_partner();
        let (Some(split_causing), Some(split_partner)) = (split_causing, split_partner) else { return };

        HyperEdgeSegmentDependency::create_and_add_critical(segment, &split_causing);
        HyperEdgeSegmentDependency::create_and_add_critical(&split_causing, &split_partner);

        for other in segments {
            if !Rc::ptr_eq(other, &split_causing)
                && !Rc::ptr_eq(other, segment)
                && !Rc::ptr_eq(other, &split_partner)
            {
                routing_generator.create_dependency_if_necessary(other, segment);
                routing_generator.create_dependency_if_necessary(other, &split_partner);
            }
        }
    }

    fn compute_position_to_split_and_update_free_areas(
        segment: &HyperEdgeSegmentRef,
        free_areas: &mut Vec<FreeArea>,
        critical_conflict_threshold: f64,
    ) -> f64 {
        let mut first_possible = None;
        let mut last_possible = None;

        let (segment_start, segment_end) = {
            let segment_guard = segment.borrow();
            (segment_guard.start_coordinate(), segment_guard.end_coordinate())
        };

        for (idx, area) in free_areas.iter().enumerate() {
            if area.start_position > segment_end {
                break;
            } else if area.end_position >= segment_start {
                if first_possible.is_none() {
                    first_possible = Some(idx);
                }
                last_possible = Some(idx);
            }
        }

        let mut split_position = center_values(segment_start, segment_end);
        if let (Some(first_index), Some(last_index)) = (first_possible, last_possible) {
            let best_index = Self::choose_best_area_index(segment, free_areas, first_index, last_index);
            split_position = center_area(&free_areas[best_index]);
            Self::use_area(free_areas, best_index, critical_conflict_threshold);
        }

        split_position
    }

    fn choose_best_area_index(
        segment: &HyperEdgeSegmentRef,
        free_areas: &[FreeArea],
        from_index: usize,
        to_index: usize,
    ) -> usize {
        let mut best_area_index = from_index;

        if from_index < to_index {
            let split_segments = segment.borrow().simulate_split();
            let split_segment = split_segments.first().clone();
            let split_partner = split_segments.second().clone();

            let mut best_area = &free_areas[best_area_index];
            let mut best_rating = Self::rate_area(segment, &split_segment, &split_partner, best_area);

            for i in (from_index + 1)..=to_index {
                let curr_area = &free_areas[i];
                let curr_rating = Self::rate_area(segment, &split_segment, &split_partner, curr_area);
                if is_better(curr_area, &curr_rating, best_area, &best_rating) {
                    best_area = curr_area;
                    best_rating = curr_rating;
                    best_area_index = i;
                }
            }
        }

        best_area_index
    }

    fn rate_area(
        segment: &HyperEdgeSegmentRef,
        split_segment: &HyperEdgeSegmentRef,
        split_partner: &HyperEdgeSegmentRef,
        area: &FreeArea,
    ) -> AreaRating {
        let area_center = center_area(area);

        {
            let mut split = split_segment.borrow_mut();
            split.outgoing_connection_coordinates_mut().clear();
            split.outgoing_connection_coordinates_mut().push(area_center);
        }
        {
            let mut partner = split_partner.borrow_mut();
            partner.incoming_connection_coordinates_mut().clear();
            partner.incoming_connection_coordinates_mut().push(area_center);
        }

        let mut rating = AreaRating::new(0, 0);

        let incoming_deps = segment.borrow().incoming_segment_dependencies().clone();
        for dep in incoming_deps {
            if let Some(other_segment) = dep.borrow().source() {
                Self::update_considering_both_orderings(&mut rating, split_segment, &other_segment);
                Self::update_considering_both_orderings(&mut rating, split_partner, &other_segment);
            }
        }

        let outgoing_deps = segment.borrow().outgoing_segment_dependencies().clone();
        for dep in outgoing_deps {
            if let Some(other_segment) = dep.borrow().target() {
                Self::update_considering_both_orderings(&mut rating, split_segment, &other_segment);
                Self::update_considering_both_orderings(&mut rating, split_partner, &other_segment);
            }
        }

        rating.dependencies += 2;

        let split_by = segment.borrow().split_by();
        if let Some(split_by) = split_by {
            rating.crossings += Self::count_crossings_for_single_ordering(split_segment, &split_by);
            rating.crossings += Self::count_crossings_for_single_ordering(&split_by, split_partner);
        }

        rating
    }

    fn update_considering_both_orderings(
        rating: &mut AreaRating,
        s1: &HyperEdgeSegmentRef,
        s2: &HyperEdgeSegmentRef,
    ) {
        let crossings_s1_left = Self::count_crossings_for_single_ordering(s1, s2);
        let crossings_s2_left = Self::count_crossings_for_single_ordering(s2, s1);

        if crossings_s1_left == crossings_s2_left {
            if crossings_s1_left > 0 {
                rating.dependencies += 2;
                rating.crossings += crossings_s1_left;
            }
        } else {
            rating.dependencies += 1;
            rating.crossings += crossings_s1_left.min(crossings_s2_left);
        }
    }

    fn count_crossings_for_single_ordering(left: &HyperEdgeSegmentRef, right: &HyperEdgeSegmentRef) -> i32 {
        OrthogonalRoutingGenerator::count_crossings(
            left.borrow().outgoing_connection_coordinates(),
            right.borrow().start_coordinate(),
            right.borrow().end_coordinate(),
        ) + OrthogonalRoutingGenerator::count_crossings(
            right.borrow().incoming_connection_coordinates(),
            left.borrow().start_coordinate(),
            left.borrow().end_coordinate(),
        )
    }

    fn use_area(
        free_areas: &mut Vec<FreeArea>,
        used_area_index: usize,
        critical_conflict_threshold: f64,
    ) {
        let old_area = free_areas.remove(used_area_index);

        if old_area.size / 2.0 >= critical_conflict_threshold {
            let mut insert_index = used_area_index;
            let old_area_center = center_area(&old_area);

            let new_end1 = old_area_center - critical_conflict_threshold;
            if old_area.start_position <= new_end1 {
                free_areas.insert(insert_index, FreeArea::new(old_area.start_position, new_end1));
                insert_index += 1;
            }

            let new_start2 = old_area_center + critical_conflict_threshold;
            if new_start2 <= old_area.end_position {
                free_areas.insert(insert_index, FreeArea::new(new_start2, old_area.end_position));
            }
        }
    }
}

#[derive(Clone)]
struct FreeArea {
    start_position: f64,
    end_position: f64,
    size: f64,
}

impl FreeArea {
    fn new(start_position: f64, end_position: f64) -> Self {
        assert!(end_position >= start_position);
        FreeArea {
            start_position,
            end_position,
            size: end_position - start_position,
        }
    }
}

#[derive(Clone)]
struct AreaRating {
    dependencies: i32,
    crossings: i32,
}

impl AreaRating {
    fn new(dependencies: i32, crossings: i32) -> Self {
        AreaRating {
            dependencies,
            crossings,
        }
    }
}

fn center_area(area: &FreeArea) -> f64 {
    center_values(area.start_position, area.end_position)
}

fn center_values(p1: f64, p2: f64) -> f64 {
    (p1 + p2) / 2.0
}

fn is_better(curr_area: &FreeArea, curr_rating: &AreaRating, best_area: &FreeArea, best_rating: &AreaRating) -> bool {
    if curr_rating.crossings < best_rating.crossings {
        return true;
    }
    if curr_rating.crossings == best_rating.crossings {
        if curr_rating.dependencies < best_rating.dependencies {
            return true;
        }
        if curr_rating.dependencies == best_rating.dependencies {
            if curr_area.size > best_area.size {
                return true;
            }
        }
    }
    false
}

fn contains_segment(list: &[HyperEdgeSegmentRef], segment: &HyperEdgeSegmentRef) -> bool {
    list.iter().any(|item| Rc::ptr_eq(item, segment))
}
