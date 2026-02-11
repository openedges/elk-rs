use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet, VecDeque};

use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkRectangle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlapRemovalDirection {
    Up,
    Down,
    Left,
    Right,
}

pub trait RectangleStripOverlapRemovalStrategy {
    fn remove_overlaps(&self, overlap_remover: &mut RectangleStripOverlapRemover) -> f64;
}

pub struct RectangleStripOverlapRemover {
    overlap_removal_direction: OverlapRemovalDirection,
    gap_vertical: f64,
    gap_horizontal: f64,
    start_coordinate: f64,
    overlap_removal_strategy: Option<Box<dyn RectangleStripOverlapRemovalStrategy>>,
    rectangle_nodes: Vec<RectangleNode>,
}

impl RectangleStripOverlapRemover {
    pub fn create_for_direction(direction: OverlapRemovalDirection) -> Self {
        RectangleStripOverlapRemover {
            overlap_removal_direction: direction,
            gap_vertical: 5.0,
            gap_horizontal: 5.0,
            start_coordinate: 0.0,
            overlap_removal_strategy: None,
            rectangle_nodes: Vec::new(),
        }
    }

    pub fn with_gap(mut self, horizontal_gap: f64, vertical_gap: f64) -> Self {
        self.gap_horizontal = horizontal_gap;
        self.gap_vertical = vertical_gap;
        self
    }

    pub fn with_start_coordinate(mut self, coordinate: f64) -> Self {
        self.start_coordinate = coordinate;
        self
    }

    pub fn with_overlap_removal_strategy(
        mut self,
        strategy: Box<dyn RectangleStripOverlapRemovalStrategy>,
    ) -> Self {
        self.overlap_removal_strategy = Some(strategy);
        self
    }

    pub fn add_rectangle(&mut self, rectangle: &mut ElkRectangle) {
        let transformed = self.import_rectangle(rectangle);
        self.rectangle_nodes.push(RectangleNode::new(rectangle, transformed));
    }

    pub fn horizontal_gap(&self) -> f64 {
        self.gap_horizontal
    }

    pub fn vertical_gap(&self) -> f64 {
        self.gap_vertical
    }

    pub fn rectangle_nodes(&self) -> &Vec<RectangleNode> {
        &self.rectangle_nodes
    }

    pub fn rectangle_nodes_mut(&mut self) -> &mut Vec<RectangleNode> {
        &mut self.rectangle_nodes
    }

    pub fn remove_overlaps(&mut self) -> f64 {
        if self.overlap_removal_strategy.is_none() {
            self.overlap_removal_strategy = Some(Box::new(GreedyRectangleStripOverlapRemover));
        }

        self.rectangle_nodes
            .sort_by(|a, b| a.rectangle.x.total_cmp(&b.rectangle.x));

        self.compute_overlaps();

        let strategy = self
            .overlap_removal_strategy
            .take()
            .unwrap_or_else(|| Box::new(GreedyRectangleStripOverlapRemover));
        let strip_size = strategy.remove_overlaps(self);
        self.overlap_removal_strategy = Some(strategy);

        for node in &self.rectangle_nodes {
            self.export_rectangle(node, strip_size);
        }

        strip_size
    }

    fn import_rectangle(&self, rectangle: &ElkRectangle) -> ElkRectangle {
        match self.overlap_removal_direction {
            OverlapRemovalDirection::Up | OverlapRemovalDirection::Down => *rectangle,
            OverlapRemovalDirection::Left | OverlapRemovalDirection::Right => {
                ElkRectangle::with_values(rectangle.y, 0.0, rectangle.height, rectangle.width)
            }
        }
    }

    fn export_rectangle(&self, node: &RectangleNode, _strip_size: f64) {
        let rect = node.rectangle;
        let original = node.original_rectangle;

        unsafe {
            let original_rect = &mut *original;
            match self.overlap_removal_direction {
                OverlapRemovalDirection::Up => {
                    original_rect.y = self.start_coordinate - rect.height - rect.y;
                }
                OverlapRemovalDirection::Down => {
                    original_rect.y = rect.y + self.start_coordinate;
                }
                OverlapRemovalDirection::Left => {
                    original_rect.x = self.start_coordinate - rect.height - rect.y;
                }
                OverlapRemovalDirection::Right => {
                    original_rect.x = self.start_coordinate + rect.y;
                }
            }
        }
    }

    fn compute_overlaps(&mut self) {
        let len = self.rectangle_nodes.len();
        for node in &mut self.rectangle_nodes {
            node.overlapping_nodes.clear();
        }

        let mut intersecting_nodes: BTreeSet<RightBorderKey> = BTreeSet::new();
        for current_index in 0..len {
            let scanline_pos = self.rectangle_nodes[current_index].rectangle.x;

            loop {
                let first = intersecting_nodes.iter().next().copied();
                match first {
                    Some(node) if node.right < scanline_pos => {
                        intersecting_nodes.remove(&node);
                    }
                    _ => break,
                }
            }

            let intersecting_indices: Vec<usize> =
                intersecting_nodes.iter().map(|node| node.index).collect();

            let (left, right) = self.rectangle_nodes.split_at_mut(current_index);
            let (current_node, _) = right
                .split_first_mut()
                .expect("current index should be in bounds");

            for other_index in intersecting_indices {
                current_node.overlapping_nodes.push(other_index);
                left[other_index].overlapping_nodes.push(current_index);
            }

            let curr_rect = current_node.rectangle;
            intersecting_nodes.insert(RightBorderKey {
                right: curr_rect.x + curr_rect.width,
                index: current_index,
            });
        }
    }
}

pub struct RectangleNode {
    original_rectangle: *mut ElkRectangle,
    rectangle: ElkRectangle,
    overlapping_nodes: Vec<usize>,
}

impl RectangleNode {
    fn new(original_rectangle: &mut ElkRectangle, rectangle: ElkRectangle) -> Self {
        RectangleNode {
            original_rectangle,
            rectangle,
            overlapping_nodes: Vec::new(),
        }
    }

    pub fn rectangle(&self) -> &ElkRectangle {
        &self.rectangle
    }

    pub fn rectangle_mut(&mut self) -> &mut ElkRectangle {
        &mut self.rectangle
    }

    pub fn overlapping_nodes(&self) -> &Vec<usize> {
        &self.overlapping_nodes
    }

}

#[derive(Clone, Copy, Debug)]
struct RightBorderKey {
    right: f64,
    index: usize,
}

impl PartialEq for RightBorderKey {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.right.total_cmp(&other.right) == Ordering::Equal
    }
}

impl Eq for RightBorderKey {}

impl PartialOrd for RightBorderKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RightBorderKey {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.right.total_cmp(&other.right) {
            Ordering::Equal => self.index.cmp(&other.index),
            ordering => ordering,
        }
    }
}

pub struct GreedyRectangleStripOverlapRemover;

impl RectangleStripOverlapRemovalStrategy for GreedyRectangleStripOverlapRemover {
    fn remove_overlaps(&self, overlap_remover: &mut RectangleStripOverlapRemover) -> f64 {
        let vertical_gap = overlap_remover.vertical_gap();
        let mut already_placed: HashSet<usize> = HashSet::new();
        let mut strip_size: f64 = 0.0;

        let order: VecDeque<usize> = (0..overlap_remover.rectangle_nodes.len()).collect();
        for node_index in order {
            let (curr_height, overlapping) = {
                let node = &overlap_remover.rectangle_nodes[node_index];
                (node.rectangle.height, node.overlapping_nodes.clone())
            };

            let mut y_pos = 0.0;
            let mut overlaps = overlapping;
            overlaps.sort_by(|a, b| {
                let rect_a = overlap_remover.rectangle_nodes[*a].rectangle;
                let rect_b = overlap_remover.rectangle_nodes[*b].rectangle;
                rect_a.y.total_cmp(&rect_b.y)
            });

            for overlap_index in overlaps {
                if !already_placed.contains(&overlap_index) {
                    continue;
                }
                let overlap_rect = overlap_remover.rectangle_nodes[overlap_index].rectangle;
                if y_pos < overlap_rect.y + overlap_rect.height + vertical_gap
                    && y_pos + curr_height + vertical_gap > overlap_rect.y
                {
                    y_pos = overlap_rect.y + overlap_rect.height + vertical_gap;
                }
            }

            if let Some(rect) = overlap_remover.rectangle_nodes.get_mut(node_index) {
                rect.rectangle.y = y_pos;
                already_placed.insert(node_index);
                strip_size = strip_size.max(rect.rectangle.y + rect.rectangle.height);
            }
        }

        strip_size
    }
}
