use std::cell::RefCell;
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, VecDeque};
use std::rc::Rc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use crate::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use crate::org::eclipse::elk::core::math::{ElkPadding, KVector};
use crate::org::eclipse::elk::core::options::{BoxLayouterOptions, PackingMode};
use crate::org::eclipse::elk::core::util::{ElkUtil, IElkProgressMonitor};

#[derive(Clone, Default)]
pub struct BoxLayoutProvider;

impl BoxLayoutProvider {
    pub const DEF_ASPECT_RATIO: f64 = 1.3;

    pub fn new() -> Self {
        BoxLayoutProvider
    }
}

impl IGraphLayoutEngine for BoxLayoutProvider {
    fn layout(&mut self, layout_node: &ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Box layout", 2.0);

        let (obj_spacing, padding, expand_nodes, interactive, packing_mode) =
            with_node_properties_mut(layout_node, |props| {
                let spacing = props
                    .get_property(BoxLayouterOptions::SPACING_NODE_NODE)
                    .unwrap_or(0.0);
                let padding = props
                    .get_property(BoxLayouterOptions::PADDING)
                    .unwrap_or_else(ElkPadding::new);
                let expand_nodes = props
                    .get_property(BoxLayouterOptions::EXPAND_NODES)
                    .unwrap_or(false);
                let interactive = props
                    .get_property(BoxLayouterOptions::INTERACTIVE)
                    .unwrap_or(false);
                let packing_mode = props
                    .get_property(BoxLayouterOptions::BOX_PACKING_MODE)
                    .unwrap_or(PackingMode::Simple);
                (spacing, padding, expand_nodes, interactive, packing_mode)
            });

        match packing_mode {
            PackingMode::Simple => place_boxes(
                layout_node,
                obj_spacing,
                &padding,
                expand_nodes,
                interactive,
            ),
            _ => place_boxes_grouping(layout_node, obj_spacing, &padding, expand_nodes),
        }

        progress_monitor.done();
    }
}

impl AbstractLayoutProvider for BoxLayoutProvider {}

fn place_boxes(
    parent_node: &ElkNodeRef,
    obj_spacing: f64,
    padding: &ElkPadding,
    expand_nodes: bool,
    interactive: bool,
) {
    let sorted_boxes = sort_boxes(parent_node, interactive);
    let min_size = ElkUtil::effective_min_size_constraint_for(parent_node);

    let mut aspect_ratio = with_node_properties_mut(parent_node, |props| {
        props.get_property(BoxLayouterOptions::ASPECT_RATIO)
    })
    .unwrap_or(BoxLayoutProvider::DEF_ASPECT_RATIO);
    if aspect_ratio <= 0.0 {
        aspect_ratio = BoxLayoutProvider::DEF_ASPECT_RATIO;
    }

    let parent_size = place_boxes_sorted(
        &sorted_boxes,
        obj_spacing,
        padding,
        min_size.x,
        min_size.y,
        expand_nodes,
        aspect_ratio,
    );

    ElkUtil::resize_node_with(parent_node, parent_size.x, parent_size.y, false, true);
}

fn sort_boxes(parent_node: &ElkNodeRef, interactive: bool) -> Vec<ElkNodeRef> {
    let mut sorted_boxes: Vec<ElkNodeRef> = {
        let mut parent_mut = parent_node.borrow_mut();
        parent_mut.children().iter().cloned().collect()
    };

    sorted_boxes.sort_by(|child1, child2| {
        let prio1 = with_node_properties_mut(child1, |props| {
            props
                .get_property(BoxLayouterOptions::PRIORITY)
                .unwrap_or(0)
        });
        let prio2 = with_node_properties_mut(child2, |props| {
            props
                .get_property(BoxLayouterOptions::PRIORITY)
                .unwrap_or(0)
        });

        if prio1 > prio2 {
            return Ordering::Less;
        } else if prio1 < prio2 {
            return Ordering::Greater;
        }

        if interactive {
            let (x1, y1) = node_position(child1);
            let (x2, y2) = node_position(child2);
            let cmp = y1.partial_cmp(&y2).unwrap_or(Ordering::Equal);
            if cmp != Ordering::Equal {
                return cmp;
            }
            let cmp = x1.partial_cmp(&x2).unwrap_or(Ordering::Equal);
            if cmp != Ordering::Equal {
                return cmp;
            }
        }

        let size1 = node_area(child1);
        let size2 = node_area(child2);
        size1.partial_cmp(&size2).unwrap_or(Ordering::Equal)
    });

    sorted_boxes
}

fn place_boxes_sorted(
    sorted_boxes: &[ElkNodeRef],
    min_spacing: f64,
    padding: &ElkPadding,
    min_total_width: f64,
    min_total_height: f64,
    expand_nodes: bool,
    aspect_ratio: f64,
) -> KVector {
    let mut max_row_width: f64 = 0.0;
    let mut total_area: f64 = 0.0;

    for box_node in sorted_boxes {
        ElkUtil::resize_node(box_node);
        let (width, height) = node_size(box_node);
        max_row_width = max_row_width.max(width);
        total_area += width * height;
    }

    let mean = total_area / (sorted_boxes.len() as f64);
    let stddev = area_std_dev(sorted_boxes, mean);

    total_area += (sorted_boxes.len() as f64) * stddev;
    total_area += total_area.sqrt() * (padding.bottom + padding.top);
    total_area += total_area.sqrt() * padding.right;

    max_row_width = max_row_width.max((total_area * aspect_ratio).sqrt()) + padding.left;

    let mut xpos = padding.left;
    let mut ypos = padding.top;
    let mut highest_box = 0.0;
    let mut broadest_row = padding.left + padding.right;
    let mut row_indices: Vec<usize> = vec![0];
    let mut row_heights: Vec<f64> = Vec::new();

    for (index, box_node) in sorted_boxes.iter().enumerate() {
        let (width, height) = node_size(box_node);
        if xpos + width > max_row_width {
            if expand_nodes {
                row_heights.push(highest_box);
                row_indices.push(index);
            }
            xpos = padding.left;
            ypos += highest_box + min_spacing;
            highest_box = 0.0;
            broadest_row = broadest_row.max(padding.left + padding.right + width);
        }
        set_node_location(box_node, xpos, ypos);
        broadest_row = broadest_row.max(xpos + width + padding.right);
        highest_box = highest_box.max(height);
        xpos += width + min_spacing;
    }

    broadest_row = broadest_row.max(min_total_width);
    let mut total_height = ypos + highest_box + padding.bottom;
    if total_height < min_total_height {
        highest_box += min_total_height - total_height;
        total_height = min_total_height;
    }

    if expand_nodes {
        xpos = padding.left;
        row_indices.push(sorted_boxes.len());
        row_heights.push(highest_box);

        let mut row_index_iter = row_indices.iter();
        let mut row_height_iter = row_heights.iter();
        let mut next_row_index = *row_index_iter.next().unwrap_or(&sorted_boxes.len());
        let mut row_height = 0.0;

        for index in 0..sorted_boxes.len() {
            if index == next_row_index {
                xpos = padding.left;
                row_height = *row_height_iter.next().unwrap_or(&0.0);
                next_row_index = *row_index_iter.next().unwrap_or(&sorted_boxes.len());
            }
            let box_node = &sorted_boxes[index];
            let old_height = node_height(box_node);
            set_node_height(box_node, row_height);
            let new_height = row_height;

            if index + 1 == next_row_index {
                let new_width = broadest_row - xpos - padding.right;
                let old_width = node_width(box_node);
                set_node_width(box_node, new_width);
                let new_size = KVector::with_values(new_width, new_height);
                let old_size = KVector::with_values(old_width, old_height);
                ElkUtil::translate((box_node, &new_size, &old_size));
            }
            xpos += node_width(box_node) + min_spacing;
        }
    }

    KVector::with_values(broadest_row, total_height)
}

fn area_std_dev(boxes: &[ElkNodeRef], mean: f64) -> f64 {
    let mut variance = 0.0;
    for box_node in boxes {
        let area = node_area(box_node);
        variance += (area - mean).powi(2);
    }
    (variance / ((boxes.len() as f64) - 1.0)).sqrt()
}

fn place_boxes_grouping(
    parent_node: &ElkNodeRef,
    obj_spacing: f64,
    padding: &ElkPadding,
    expand_nodes: bool,
) {
    let mut min_size = with_node_properties_mut(parent_node, |props| {
        props
            .get_property(BoxLayouterOptions::NODE_SIZE_MINIMUM)
            .unwrap_or_else(KVector::new)
    });

    min_size.x = (min_size.x - padding.left - padding.right).max(0.0);
    min_size.y = (min_size.y - padding.top - padding.bottom).max(0.0);

    let mut aspect_ratio = with_node_properties_mut(parent_node, |props| {
        props.get_property(BoxLayouterOptions::ASPECT_RATIO)
    })
    .unwrap_or(BoxLayoutProvider::DEF_ASPECT_RATIO);
    if aspect_ratio <= 0.0 {
        aspect_ratio = BoxLayoutProvider::DEF_ASPECT_RATIO;
    }

    let groups: Vec<GroupRef> = {
        let mut parent_mut = parent_node.borrow_mut();
        parent_mut
            .children()
            .iter()
            .cloned()
            .map(Group::new_node)
            .collect()
    };

    let mode = with_node_properties_mut(parent_node, |props| {
        props
            .get_property(BoxLayouterOptions::BOX_PACKING_MODE)
            .unwrap_or(PackingMode::Simple)
    });

    let to_be_placed = match mode {
        PackingMode::GroupInc => merge_and_place_inc(
            groups,
            obj_spacing,
            min_size.x,
            min_size.y,
            expand_nodes,
            aspect_ratio,
        ),
        PackingMode::GroupDec => merge_and_place_dec(
            groups,
            obj_spacing,
            min_size.x,
            min_size.y,
            expand_nodes,
            aspect_ratio,
        ),
        _ => merge_and_place_mixed(
            groups,
            obj_spacing,
            min_size.x,
            min_size.y,
            expand_nodes,
            aspect_ratio,
        ),
    };

    let final_group = Group::new_groups(to_be_placed);
    let parent_size = place_inner_boxes(
        &final_group,
        obj_spacing,
        padding,
        min_size.x,
        min_size.y,
        expand_nodes,
        aspect_ratio,
    );

    ElkUtil::resize_node_with(parent_node, parent_size.x, parent_size.y, false, true);
}

fn place_inner_boxes(
    group: &GroupRef,
    min_spacing: f64,
    padding: &ElkPadding,
    min_total_width: f64,
    min_total_height: f64,
    expand_nodes: bool,
    aspect_ratio: f64,
) -> KVector {
    let groups = group_groups(group);
    let mut max_row_width: f64 = 0.0;
    let mut total_area: f64 = 0.0;

    for box_group in &groups {
        if let Some(node) = group_node(box_group) {
            ElkUtil::resize_node(&node);
        }
        let width = group_width(box_group);
        let height = group_height(box_group);
        max_row_width = max_row_width.max(width);
        total_area += width * height;
    }

    let mean = total_area / (groups.len() as f64);
    let stddev = area_std_dev_groups(&groups, mean);
    let sd_influence = 1.0;
    total_area += (groups.len() as f64) * sd_influence * stddev;

    max_row_width = max_row_width.max((total_area * aspect_ratio).sqrt()) + padding.left;

    let mut xpos = padding.left;
    let mut ypos = padding.top;
    let mut highest_box = 0.0;
    let mut broadest_row = padding.left + padding.right;
    let mut row_indices: Vec<usize> = vec![0];
    let mut row_heights: Vec<f64> = Vec::new();
    let mut last: Option<GroupRef> = None;
    let mut bottoms: Vec<GroupRef> = Vec::new();

    for (index, box_group) in groups.iter().enumerate() {
        let width = group_width(box_group);
        let height = group_height(box_group);
        if xpos + width > max_row_width {
            if expand_nodes {
                row_heights.push(highest_box);
                row_indices.push(index);
                if let Some(last_group) = last.take() {
                    group_right_push(group, last_group);
                }
                bottoms.clear();
            }
            xpos = padding.left;
            ypos += highest_box + min_spacing;
            highest_box = 0.0;
            broadest_row = broadest_row.max(padding.left + padding.right + width);
        }
        bottoms.push(box_group.clone());
        group_translate(box_group, xpos, ypos);
        broadest_row = broadest_row.max(xpos + width + padding.right);
        highest_box = highest_box.max(height);
        xpos += width + min_spacing;
        last = Some(box_group.clone());
    }

    group_bottom_extend(group, &bottoms);
    if let Some(last_bottom) = bottoms.last() {
        group_right_push(group, last_bottom.clone());
    }

    broadest_row = broadest_row.max(min_total_width);
    let mut total_height = ypos + highest_box + padding.bottom;
    if total_height < min_total_height {
        highest_box += min_total_height - total_height;
        total_height = min_total_height;
    }

    if expand_nodes {
        xpos = padding.left;
        row_indices.push(groups.len());
        row_heights.push(highest_box);
        let mut row_index_iter = row_indices.iter();
        let mut row_height_iter = row_heights.iter();
        let mut next_row_index = *row_index_iter.next().unwrap_or(&groups.len());
        let mut row_height = 0.0;

        for index in 0..groups.len() {
            if index == next_row_index {
                xpos = padding.left;
                row_height = *row_height_iter.next().unwrap_or(&0.0);
                next_row_index = *row_index_iter.next().unwrap_or(&groups.len());
            }
            let box_group = &groups[index];
            group_set_height(box_group, row_height);
            if index + 1 == next_row_index {
                let new_width = broadest_row - xpos - padding.right;
                let old_width = group_width(box_group);
                group_set_width(box_group, new_width);
                group_translate_inner_nodes(box_group, (new_width - old_width) / 2.0, 0.0);
            }
            xpos += group_width(box_group) + min_spacing;
        }
    }

    KVector::with_values(broadest_row, total_height)
}

fn area_std_dev_groups(groups: &[GroupRef], mean: f64) -> f64 {
    let mut variance = 0.0;
    for group in groups {
        variance += (group_area(group) - mean).powi(2);
    }
    (variance / ((groups.len() as f64) - 1.0)).sqrt()
}

fn merge_and_place_dec(
    mut groups: Vec<GroupRef>,
    obj_spacing: f64,
    min_width: f64,
    min_height: f64,
    expand_nodes: bool,
    _aspect_ratio: f64,
) -> Vec<GroupRef> {
    groups.sort_by(|g1, g2| {
        group_area(g2)
            .partial_cmp(&group_area(g1))
            .unwrap_or(Ordering::Equal)
    });

    let mut box_queue: VecDeque<GroupRef> = VecDeque::from(groups);
    let mut to_be_placed: Vec<GroupRef> = Vec::new();
    let mut maybe_group: Vec<GroupRef> = Vec::new();
    let mut box_to_beat: Option<GroupRef> = None;
    let mut collected_area = 0.0;

    while let Some(box_group) = box_queue.pop_front() {
        let box_area = group_area(&box_group);
        let beat_area = box_to_beat.as_ref().map(group_area);
        if box_to_beat.is_none() || beat_area.unwrap_or(0.0) / 2.0 < box_area {
            box_to_beat = Some(box_group.clone());
            to_be_placed.push(box_group);
        } else {
            collected_area += box_area;
            maybe_group.push(box_group);
            if maybe_group.len() > 1
                && (collected_area > beat_area.unwrap_or(0.0) / 2.0 || box_queue.is_empty())
            {
                let inner_group = Group::new_groups(maybe_group.clone());
                let inner_aspect_ratio = group_width(box_to_beat.as_ref().unwrap())
                    / group_height(box_to_beat.as_ref().unwrap());
                let group_size = place_inner_boxes(
                    &inner_group,
                    obj_spacing,
                    &ElkPadding::new(),
                    min_width,
                    min_height,
                    expand_nodes,
                    inner_aspect_ratio,
                );
                {
                    let mut inner_mut = inner_group.borrow_mut();
                    inner_mut.size.reset().add(&group_size);
                }
                box_to_beat = Some(inner_group.clone());
                to_be_placed.push(inner_group);
                collected_area = 0.0;
                maybe_group.clear();
            }
        }
    }

    to_be_placed.extend(maybe_group);
    to_be_placed
}

fn merge_and_place_mixed(
    groups: Vec<GroupRef>,
    obj_spacing: f64,
    min_width: f64,
    min_height: f64,
    expand_nodes: bool,
    _aspect_ratio: f64,
) -> Vec<GroupRef> {
    let mut cum_area = vec![0.0; groups.len()];
    let mut heap: BinaryHeap<Reverse<GroupEntry>> = BinaryHeap::new();
    let mut seq = 0;
    for group in groups {
        let entry = GroupEntry::new(group, seq);
        seq += 1;
        heap.push(Reverse(entry));
    }

    let mut index = 0usize;
    let mut to_be_placed: Vec<GroupRef> = Vec::new();

    while let Some(peek) = heap.peek() {
        let box_group = peek.0.group.clone();
        let box_area = group_area(&box_group);

        if index > 1 && (box_area / 2.0 > cum_area[0]) {
            let mut an_index = 0usize;
            while an_index < to_be_placed.len() - 1 && (box_area / 2.0 > cum_area[an_index]) {
                an_index += 1;
            }

            let select: Vec<GroupRef> = to_be_placed[..=an_index].to_vec();
            let inner_group = Group::new_groups(select);
            let inner_aspect_ratio = group_width(&box_group) / group_height(&box_group);
            let group_size = place_inner_boxes(
                &inner_group,
                obj_spacing,
                &ElkPadding::new(),
                min_width,
                min_height,
                expand_nodes,
                inner_aspect_ratio,
            );
            {
                let mut inner_mut = inner_group.borrow_mut();
                inner_mut.size.reset().add(&group_size);
            }

            heap.push(Reverse(GroupEntry::new(inner_group, seq)));
            seq += 1;
            for group in to_be_placed.iter().skip(an_index + 1) {
                heap.push(Reverse(GroupEntry::new(group.clone(), seq)));
                seq += 1;
            }
            to_be_placed.clear();
            index = 0;
            cum_area.fill(0.0);
        } else {
            let entry = heap.pop().unwrap().0;
            if index > 0 {
                cum_area[index] = cum_area[index - 1];
            }
            cum_area[index] += entry.area;
            index += 1;
            to_be_placed.push(entry.group);
        }
    }

    to_be_placed
}

fn merge_and_place_inc(
    mut groups: Vec<GroupRef>,
    obj_spacing: f64,
    min_width: f64,
    min_height: f64,
    expand_nodes: bool,
    _aspect_ratio: f64,
) -> Vec<GroupRef> {
    groups.sort_by(|g1, g2| {
        group_area(g1)
            .partial_cmp(&group_area(g2))
            .unwrap_or(Ordering::Equal)
    });

    let mut to_be_placed: Vec<GroupRef> = Vec::new();
    let mut common_area = 0.0;

    for group in groups {
        if !to_be_placed.is_empty() && group_area(&group) > (common_area * 2.0) {
            let merged = Group::new_groups(to_be_placed.clone());
            let inner_aspect_ratio = group_width(&group) / group_height(&group);
            let group_size = place_inner_boxes(
                &merged,
                obj_spacing,
                &ElkPadding::new(),
                min_width,
                min_height,
                expand_nodes,
                inner_aspect_ratio,
            );
            {
                let mut merged_mut = merged.borrow_mut();
                merged_mut.size.reset().add(&group_size);
            }
            to_be_placed.clear();
            to_be_placed.push(merged.clone());
            to_be_placed.push(group.clone());
            common_area = group_area(&merged) + group_area(&group);
        } else {
            common_area += group_area(&group);
            to_be_placed.push(group);
        }
    }

    to_be_placed
}

#[derive(Clone)]
struct GroupEntry {
    area: f64,
    seq: usize,
    group: GroupRef,
}

impl GroupEntry {
    fn new(group: GroupRef, seq: usize) -> Self {
        GroupEntry {
            area: group_area(&group),
            seq,
            group,
        }
    }
}

impl PartialEq for GroupEntry {
    fn eq(&self, other: &Self) -> bool {
        self.area == other.area && self.seq == other.seq
    }
}

impl Eq for GroupEntry {}

impl PartialOrd for GroupEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GroupEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        match self
            .area
            .partial_cmp(&other.area)
            .unwrap_or(Ordering::Equal)
        {
            Ordering::Equal => self.seq.cmp(&other.seq),
            ord => ord,
        }
    }
}

type GroupRef = Rc<RefCell<Group>>;

struct Group {
    node: Option<ElkNodeRef>,
    groups: Vec<GroupRef>,
    size: KVector,
    bottom: Vec<GroupRef>,
    right: Vec<GroupRef>,
}

impl Group {
    fn new_node(node: ElkNodeRef) -> GroupRef {
        {
            let mut node_mut = node.borrow_mut();
            node_mut.connectable().shape().set_location(0.0, 0.0);
        }
        Rc::new(RefCell::new(Group {
            node: Some(node),
            groups: Vec::new(),
            size: KVector::new(),
            bottom: Vec::new(),
            right: Vec::new(),
        }))
    }

    fn new_groups(groups: Vec<GroupRef>) -> GroupRef {
        Rc::new(RefCell::new(Group {
            node: None,
            groups,
            size: KVector::new(),
            bottom: Vec::new(),
            right: Vec::new(),
        }))
    }
}

fn group_node(group: &GroupRef) -> Option<ElkNodeRef> {
    group.borrow().node.clone()
}

fn group_groups(group: &GroupRef) -> Vec<GroupRef> {
    group.borrow().groups.clone()
}

fn group_right_push(group: &GroupRef, value: GroupRef) {
    group.borrow_mut().right.push(value);
}

fn group_bottom_extend(group: &GroupRef, values: &[GroupRef]) {
    group.borrow_mut().bottom.extend_from_slice(values);
}

fn group_area(group: &GroupRef) -> f64 {
    group_width(group) * group_height(group)
}

fn group_width(group: &GroupRef) -> f64 {
    let group_ref = group.borrow();
    if let Some(node) = &group_ref.node {
        node_width(node)
    } else {
        group_ref.size.x
    }
}

fn group_height(group: &GroupRef) -> f64 {
    let group_ref = group.borrow();
    if let Some(node) = &group_ref.node {
        node_height(node)
    } else {
        group_ref.size.y
    }
}

fn group_set_width(group: &GroupRef, width: f64) {
    let current = group_width(group);
    debug_assert!(width > current);
    if let Some(node) = group_node(group) {
        set_node_width(&node, width);
    } else {
        let delta = width - current;
        let right = group.borrow().right.clone();
        for group_ref in right {
            let new_width = group_width(&group_ref) + delta;
            group_set_width(&group_ref, new_width);
        }
    }
}

fn group_set_height(group: &GroupRef, height: f64) {
    let current = group_height(group);
    debug_assert!(height > current);
    if let Some(node) = group_node(group) {
        set_node_height(&node, height);
    } else {
        let delta = height - current;
        let bottom = group.borrow().bottom.clone();
        for group_ref in bottom {
            let new_height = group_height(&group_ref) + delta;
            group_set_height(&group_ref, new_height);
        }
    }
}

fn group_translate(group: &GroupRef, x: f64, y: f64) {
    if let Some(node) = group_node(group) {
        let (nx, ny) = node_position(&node);
        set_node_location(&node, nx + x, ny + y);
    } else {
        let groups = group_groups(group);
        for inner in groups {
            group_translate(&inner, x, y);
        }
    }
}

fn group_translate_inner_nodes(group: &GroupRef, x: f64, y: f64) {
    if let Some(node) = group_node(group) {
        ElkUtil::translate((&node, x, y));
    } else {
        let groups = group_groups(group);
        for inner in groups {
            group_translate_inner_nodes(&inner, x, y);
        }
    }
}

fn node_area(node: &ElkNodeRef) -> f64 {
    let (width, height) = node_size(node);
    width * height
}

fn node_size(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
}

fn node_width(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().width()
}

fn node_height(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().height()
}

fn set_node_width(node: &ElkNodeRef, width: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_width(width);
}

fn set_node_height(node: &ElkNodeRef, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_height(height);
}

fn node_position(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y())
}

fn set_node_location(node: &ElkNodeRef, x: f64, y: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_location(x, y);
}

fn with_node_properties_mut<R>(
    node: &ElkNodeRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    f(props)
}
