use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::compaction::oned::compare_fuzzy;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::compaction::oned::{
    CGraph, CGraphRef, CGroup, CNode, CNodeRef, ISpacingsHandler, OneDimensionalCompactor,
    QuadraticConstraintCalculation,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkRectangle, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{Direction, EdgeRouting, PortSide};
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LGraph, LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{
    ConstraintCalculationStrategy, GraphCompactionStrategy, InternalProperties, LayeredOptions,
    Spacings,
};
use crate::org::eclipse::elk::alg::layered::p5edges::splines::SplineSegmentRef;

#[derive(Default)]
pub struct HorizontalGraphCompactor;

impl ILayoutProcessor<LGraph> for HorizontalGraphCompactor {
    fn process(
        &mut self,
        layered_graph: &mut LGraph,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        let strategy = layered_graph
            .get_property(LayeredOptions::COMPACTION_POST_COMPACTION_STRATEGY)
            .unwrap_or(GraphCompactionStrategy::None);
        if strategy == GraphCompactionStrategy::None {
            return;
        }

        let edge_routing = layered_graph
            .get_property(LayeredOptions::EDGE_ROUTING)
            .unwrap_or(EdgeRouting::Undefined);
        if edge_routing != EdgeRouting::Orthogonal && edge_routing != EdgeRouting::Splines {
            return;
        }

        progress_monitor.begin("Horizontal Compaction", 1.0);

        let spacings = layered_graph.get_property(InternalProperties::SPACINGS);
        let constraint_strategy = layered_graph
            .get_property(LayeredOptions::COMPACTION_POST_COMPACTION_CONSTRAINTS)
            .unwrap_or(ConstraintCalculationStrategy::Scanline);

        let mut context = CompactionContext::new(layered_graph, edge_routing);
        context.transform_nodes();
        match edge_routing {
            EdgeRouting::Orthogonal => context.transform_edges_orthogonal(),
            EdgeRouting::Splines => context.transform_edges_splines(),
            _ => {}
        }
        context.compact(strategy, constraint_strategy, spacings);
        context.apply_layout(layered_graph);

        progress_monitor.done();
    }
}

#[derive(Clone)]
struct CommentOffset {
    comment: LNodeRef,
    anchor: LNodeRef,
    offset: KVector,
}

#[derive(Clone)]
struct VerticalSegment {
    segment_id: usize,
    joined_segment_ids: HashSet<usize>,
    constraint_target_ids: HashSet<usize>,
    hitbox: ElkRectangle,
    represented_edges: Vec<LEdgeRef>,
    represented_edge_keys: HashSet<usize>,
    affected_bends: Vec<(LEdgeRef, usize)>,
    affected_spline_segments: Vec<SplineSegmentRef>,
    potential_group_parents: Vec<CNodeRef>,
}

#[derive(Clone)]
struct DirectionLocks {
    left: bool,
    right: bool,
}

enum CNodeOrigin {
    Node(LNodeRef),
    Segment(usize),
}

struct CompactionContext {
    edge_routing: EdgeRouting,
    c_graph: CGraphRef,
    nodes: Vec<LNodeRef>,
    node_to_cnode: HashMap<usize, CNodeRef>,
    cnode_origin: HashMap<usize, CNodeOrigin>,
    segments: Vec<VerticalSegment>,
    cnode_to_segment: HashMap<usize, usize>,
    connection_locks: HashMap<usize, DirectionLocks>,
    comment_offsets: Vec<CommentOffset>,
    next_segment_id: usize,
}

impl CompactionContext {
    fn new(graph: &LGraph, edge_routing: EdgeRouting) -> Self {
        let has_edges = graph.layers().iter().any(|layer| {
            layer.lock().ok().is_some_and(|layer| {
                layer.nodes().iter().any(|node| {
                    node.lock()
                        .ok()
                        .is_some_and(|node| !node.connected_edges().is_empty())
                })
            })
        });

        let mut directions = vec![Direction::Left, Direction::Right];
        if !has_edges {
            directions.extend([Direction::Up, Direction::Down]);
        }

        let mut nodes = Vec::new();
        for layer in graph.layers() {
            if let Ok(layer_guard) = layer.lock() {
                nodes.extend(layer_guard.nodes().iter().cloned());
            }
        }

        Self {
            edge_routing,
            c_graph: CGraph::new(directions),
            nodes,
            node_to_cnode: HashMap::new(),
            cnode_origin: HashMap::new(),
            segments: Vec::new(),
            cnode_to_segment: HashMap::new(),
            connection_locks: HashMap::new(),
            comment_offsets: Vec::new(),
            next_segment_id: 0,
        }
    }

    fn transform_nodes(&mut self) {
        let layers = self.layer_nodes();
        for node in layers {
            if self.capture_comment_offset_if_needed(&node) {
                continue;
            }

            let (hitbox, incoming_count, outgoing_count, node_type) =
                if let Ok(mut node_guard) = node.lock() {
                    let margin = node_guard.margin().clone();
                    let shape = node_guard.shape();
                    let pos = *shape.position_ref();
                    let size = *shape.size_ref();
                    (
                        ElkRectangle::with_values(
                            pos.x - margin.left,
                            pos.y - margin.top,
                            size.x + margin.left + margin.right,
                            size.y + margin.top + margin.bottom,
                        ),
                        node_guard.incoming_edges().len(),
                        node_guard.outgoing_edges().len(),
                        node_guard.node_type(),
                    )
                } else {
                    continue;
                };

            let c_node = CNode::create(&self.c_graph, hitbox);
            CGroup::create(&self.c_graph, std::slice::from_ref(&c_node));

            let node_key = arc_key(&node);
            let cnode_key = rc_key(&c_node);
            self.node_to_cnode.insert(node_key, c_node.clone());
            self.cnode_origin
                .insert(cnode_key, CNodeOrigin::Node(node.clone()));

            let mut locks = DirectionLocks {
                left: false,
                right: false,
            };
            if node_type != NodeType::ExternalPort {
                let difference = incoming_count as isize - outgoing_count as isize;
                if difference < 0 {
                    locks.left = true;
                } else if difference > 0 {
                    locks.right = true;
                }
            }
            self.connection_locks.insert(cnode_key, locks);
        }
    }

    fn transform_edges_orthogonal(&mut self) {
        let segments = self.collect_vertical_segments_orthogonal();
        self.register_segments(segments);
    }

    fn transform_edges_splines(&mut self) {
        let segments = self.collect_vertical_segments_splines();
        self.register_segments(segments);
    }

    fn register_segments(&mut self, mut segments: Vec<VerticalSegment>) {
        if segments.is_empty() {
            return;
        }

        segments.sort_by(|a, b| {
            let x_cmp = fuzzy_cmp(a.hitbox.x, b.hitbox.x);
            if x_cmp != std::cmp::Ordering::Equal {
                return x_cmp;
            }
            a.hitbox
                .y
                .partial_cmp(&b.hitbox.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        self.segments = self.merge_segments(segments);

        let mut segment_id_to_cnode: HashMap<usize, CNodeRef> = HashMap::new();
        for (segment_index, segment) in self.segments.iter().enumerate() {
            let c_node = CNode::create(&self.c_graph, segment.hitbox);
            if let Some(parent) = segment.potential_group_parents.first() {
                if let Some(group) = parent.borrow().group() {
                    CGroup::add_c_node(&group, &c_node);
                }
            }

            let cnode_key = rc_key(&c_node);
            self.cnode_origin
                .insert(cnode_key, CNodeOrigin::Segment(segment_index));
            self.cnode_to_segment.insert(cnode_key, segment_index);
            for segment_id in &segment.joined_segment_ids {
                segment_id_to_cnode.insert(*segment_id, c_node.clone());
            }

            let mut incoming_ports: HashSet<usize> = HashSet::new();
            let mut outgoing_ports: HashSet<usize> = HashSet::new();
            for edge in &segment.represented_edges {
                if let Ok(edge_guard) = edge.lock() {
                    if let Some(source) = edge_guard.source() {
                        incoming_ports.insert(arc_key(&source));
                    }
                    if let Some(target) = edge_guard.target() {
                        outgoing_ports.insert(arc_key(&target));
                    }
                }
            }
            let difference = incoming_ports.len() as isize - outgoing_ports.len() as isize;
            let mut locks = DirectionLocks {
                left: false,
                right: false,
            };
            if difference < 0 {
                locks.left = true;
            } else if difference > 0 {
                locks.right = true;
            }
            self.connection_locks.insert(cnode_key, locks);
        }

        self.add_predefined_constraints(&segment_id_to_cnode);
    }

    fn add_predefined_constraints(&mut self, segment_id_to_cnode: &HashMap<usize, CNodeRef>) {
        let mut unique_pairs: HashSet<(usize, usize)> = HashSet::new();
        let mut constraints = Vec::new();

        for segment in &self.segments {
            let Some(source_cnode) = segment_id_to_cnode.get(&segment.segment_id).cloned() else {
                continue;
            };
            for target_segment_id in &segment.constraint_target_ids {
                let Some(target_cnode) = segment_id_to_cnode.get(target_segment_id).cloned() else {
                    continue;
                };
                if Rc::ptr_eq(&source_cnode, &target_cnode) {
                    continue;
                }
                let key = (rc_key(&source_cnode), rc_key(&target_cnode));
                if unique_pairs.insert(key) {
                    constraints.push((source_cnode.clone(), target_cnode));
                }
            }
        }

        if !constraints.is_empty() {
            self.c_graph
                .borrow_mut()
                .predefined_horizontal_constraints
                .extend(constraints);
        }
    }

    fn compact(
        &mut self,
        strategy: GraphCompactionStrategy,
        constraint_strategy: ConstraintCalculationStrategy,
        spacings: Option<Spacings>,
    ) {
        let mut compactor = OneDimensionalCompactor::new(self.c_graph.clone());
        let spacing_handler =
            SpecialSpacingsHandler::new(&self.cnode_origin, &self.segments, spacings);
        compactor.set_spacings_handler(Box::new(spacing_handler));

        match constraint_strategy {
            ConstraintCalculationStrategy::Quadratic => {
                compactor.set_constraint_algorithm(Box::new(QuadraticConstraintCalculation));
            }
            ConstraintCalculationStrategy::Scanline => {
                // Java uses EdgeAwareScanlineConstraintCalculation here. Until the edge-aware
                // variant is ported, quadratic constraints keep zero-sized node constraints stable.
                compactor.set_constraint_algorithm(Box::new(QuadraticConstraintCalculation));
            }
        }

        match strategy {
            GraphCompactionStrategy::None => {}
            GraphCompactionStrategy::Left => {
                compactor.compact();
            }
            GraphCompactionStrategy::Right => {
                compactor.change_direction(Direction::Right).compact();
            }
            GraphCompactionStrategy::LeftRightConstraintLocking => {
                compactor
                    .compact()
                    .change_direction(Direction::Right)
                    .set_lock_function(Some(Box::new(|node: &CNodeRef, _dir: Direction| {
                        node.borrow()
                            .group()
                            .is_some_and(|group| group.borrow().out_degree_real == 0)
                    })))
                    .compact();
            }
            GraphCompactionStrategy::LeftRightConnectionLocking => {
                let lock_map = self.connection_locks.clone();
                compactor
                    .compact()
                    .change_direction(Direction::Right)
                    .set_lock_function(Some(Box::new(move |node: &CNodeRef, dir: Direction| {
                        let key = rc_key(node);
                        let Some(locks) = lock_map.get(&key) else {
                            return false;
                        };
                        match dir {
                            Direction::Left => locks.left,
                            Direction::Right => locks.right,
                            _ => false,
                        }
                    })))
                    .compact();
            }
            GraphCompactionStrategy::EdgeLength => {
                // TODO: wire NetworkSimplexCompaction for full Java parity.
                compactor.compact();
            }
        }

        compactor.finish();
    }

    fn apply_layout(&self, layered_graph: &mut LGraph) {
        for c_node in &self.c_graph.borrow().c_nodes {
            let key = rc_key(c_node);
            if let Some(CNodeOrigin::Node(node)) = self.cnode_origin.get(&key) {
                if let Ok(mut node_guard) = node.lock() {
                    let left_margin = node_guard.margin().left;
                    node_guard.shape().position().x = c_node.borrow().hitbox.x + left_margin;
                }
            }
        }

        self.apply_comment_positions();
        self.apply_segment_positions();
        if self.edge_routing == EdgeRouting::Splines {
            self.apply_spline_self_loop_offsets();
            self.adjust_straight_spline_segments();
        }
        self.apply_self_loop_label_offsets();
        self.update_graph_bounds(layered_graph);
    }

    fn apply_comment_positions(&self) {
        for item in &self.comment_offsets {
            let anchor_position = item
                .anchor
                .lock()
                .ok()
                .map(|mut anchor| *anchor.shape().position_ref());
            if let (Some(anchor_position), Ok(mut comment)) = (anchor_position, item.comment.lock())
            {
                comment.shape().position().x = anchor_position.x + item.offset.x;
                comment.shape().position().y = anchor_position.y + item.offset.y;
            }
        }
    }

    fn apply_segment_positions(&self) {
        for c_node in &self.c_graph.borrow().c_nodes {
            let key = rc_key(c_node);
            let Some(segment_index) = self.cnode_to_segment.get(&key) else {
                continue;
            };
            let Some(pre) = c_node.borrow().hitbox_pre_compaction else {
                continue;
            };
            let delta_x = c_node.borrow().hitbox.x - pre.x;
            if compare_fuzzy::eq(delta_x, 0.0) {
                continue;
            }

            let segment = &self.segments[*segment_index];
            let mut adjusted: HashSet<(usize, usize)> = HashSet::new();
            for (edge, bend_index) in &segment.affected_bends {
                let edge_key = arc_key(edge);
                if !adjusted.insert((edge_key, *bend_index)) {
                    continue;
                }
                if let Ok(mut edge_guard) = edge.lock() {
                    let chain = edge_guard.bend_points();
                    if *bend_index >= chain.len() {
                        continue;
                    }
                    let mut point = chain.get(*bend_index);
                    point.x += delta_x;
                    chain.set(*bend_index, point);
                }
            }
            let mut adjusted_spline_segments: HashSet<usize> = HashSet::new();
            for spline_segment in &segment.affected_spline_segments {
                let spline_key = arc_key(spline_segment);
                if !adjusted_spline_segments.insert(spline_key) {
                    continue;
                }
                if let Ok(mut segment_guard) = spline_segment.lock() {
                    segment_guard.bounding_box.x += delta_x;
                }
            }
        }
    }

    fn apply_spline_self_loop_offsets(&self) {
        for c_node in &self.c_graph.borrow().c_nodes {
            let key = rc_key(c_node);
            let Some(CNodeOrigin::Node(node)) = self.cnode_origin.get(&key) else {
                continue;
            };
            let Some(pre) = c_node.borrow().hitbox_pre_compaction else {
                continue;
            };
            let delta_x = c_node.borrow().hitbox.x - pre.x;
            if compare_fuzzy::eq(delta_x, 0.0) {
                continue;
            }

            let outgoing = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing {
                let is_self_loop = edge
                    .lock()
                    .ok()
                    .is_some_and(|edge_guard| edge_guard.is_self_loop());
                if !is_self_loop {
                    continue;
                }
                if let Ok(mut edge_guard) = edge.lock() {
                    edge_guard.bend_points().offset(delta_x, 0.0);
                }
            }
        }
    }

    fn adjust_straight_spline_segments(&self) {
        for node in self.layer_nodes() {
            let outgoing = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing {
                let spline = edge.lock().ok().and_then(|mut edge_guard| {
                    edge_guard.get_property(InternalProperties::SPLINE_ROUTE_START)
                });
                if let Some(spline) = spline {
                    self.adjust_spline_control_points(&spline);
                }
            }
        }
    }

    fn adjust_spline_control_points(&self, spline: &[SplineSegmentRef]) {
        if spline.is_empty() {
            return;
        }

        let mut last_segment = spline[0].clone();
        if spline.len() == 1 {
            self.adjust_control_point_between_segments(&last_segment, &last_segment, 1, 0, spline);
            return;
        }

        let mut index = 1usize;
        while index < spline.len() {
            let process = last_segment
                .lock()
                .ok()
                .is_some_and(|segment| segment.initial_segment || !segment.is_straight);
            if process {
                if let Some((next_index, next_segment)) =
                    self.first_non_straight_segment(spline, index)
                {
                    self.adjust_control_point_between_segments(
                        &last_segment,
                        &next_segment,
                        index,
                        next_index,
                        spline,
                    );
                    index = next_index + 1;
                    last_segment = next_segment;
                    continue;
                }
            }
            index += 1;
        }
    }

    fn first_non_straight_segment(
        &self,
        spline: &[SplineSegmentRef],
        index: usize,
    ) -> Option<(usize, SplineSegmentRef)> {
        if index >= spline.len() {
            return None;
        }
        for (i, segment) in spline.iter().enumerate().skip(index) {
            let is_match = segment
                .lock()
                .ok()
                .is_some_and(|guard| i == spline.len() - 1 || !guard.is_straight);
            if is_match {
                return Some((i, segment.clone()));
            }
        }
        None
    }

    fn adjust_control_point_between_segments(
        &self,
        left: &SplineSegmentRef,
        right: &SplineSegmentRef,
        left_idx: usize,
        right_idx: usize,
        spline: &[SplineSegmentRef],
    ) {
        let (left_initial, left_straight, left_bbox_x, left_bbox_w, left_source_node) = left
            .lock()
            .ok()
            .map(|segment| {
                (
                    segment.initial_segment,
                    segment.is_straight,
                    segment.bounding_box.x,
                    segment.bounding_box.width,
                    segment.source_node.clone(),
                )
            })
            .unwrap_or((false, false, 0.0, 0.0, None));

        let (mut start_x, idx1) = if left_initial && left_straight {
            let source_x = left_source_node
                .as_ref()
                .and_then(|node| self.node_to_cnode.get(&arc_key(node)))
                .map(|c_node| {
                    let hitbox = c_node.borrow().hitbox;
                    hitbox.x + hitbox.width
                })
                .unwrap_or(left_bbox_x + left_bbox_w);
            (source_x, left_idx.saturating_sub(1))
        } else {
            (left_bbox_x + left_bbox_w, left_idx)
        };

        let (right_last, right_straight, right_bbox_x, right_target_node) = right
            .lock()
            .ok()
            .map(|segment| {
                (
                    segment.last_segment,
                    segment.is_straight,
                    segment.bounding_box.x,
                    segment.target_node.clone(),
                )
            })
            .unwrap_or((false, false, 0.0, None));

        let (end_x, idx2) = if right_last && right_straight {
            let target_x = right_target_node
                .as_ref()
                .and_then(|node| self.node_to_cnode.get(&arc_key(node)))
                .map(|c_node| c_node.borrow().hitbox.x)
                .unwrap_or(right_bbox_x);
            (target_x, right_idx + 1)
        } else {
            (right_bbox_x, right_idx)
        };

        let strip = end_x - start_x;
        let chunks = usize::max(2, idx2.saturating_sub(idx1)) as f64;
        let chunk = strip / chunks;

        start_x += chunk;
        for idx in idx1..idx2 {
            if let Some(segment_ref) = spline.get(idx) {
                if let Ok(mut segment) = segment_ref.lock() {
                    let width = segment.bounding_box.width;
                    segment.bounding_box.x = start_x - width / 2.0;
                }
                start_x += chunk;
            }
        }
    }

    fn apply_self_loop_label_offsets(&self) {
        for (node_key, c_node) in &self.node_to_cnode {
            let Some(CNodeOrigin::Node(node)) = self.cnode_origin.get(&rc_key(c_node)) else {
                continue;
            };
            if arc_key(node) != *node_key {
                continue;
            }
            let Some(pre) = c_node.borrow().hitbox_pre_compaction else {
                continue;
            };
            let delta_x = c_node.borrow().hitbox.x - pre.x;
            if compare_fuzzy::eq(delta_x, 0.0) {
                continue;
            }

            let outgoing = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing {
                let is_self_loop = edge
                    .lock()
                    .ok()
                    .is_some_and(|edge_guard| edge_guard.is_self_loop());
                if !is_self_loop {
                    continue;
                }
                let labels = edge
                    .lock()
                    .ok()
                    .map(|edge_guard| edge_guard.labels().clone())
                    .unwrap_or_default();
                for label in labels {
                    if let Ok(mut label_guard) = label.lock() {
                        label_guard.shape().position().x += delta_x;
                    }
                }
            }
        }
    }

    fn update_graph_bounds(&self, layered_graph: &mut LGraph) {
        if self.c_graph.borrow().c_nodes.is_empty() {
            return;
        }

        let mut top_left = KVector::with_values(f64::INFINITY, f64::INFINITY);
        let mut bottom_right = KVector::with_values(f64::NEG_INFINITY, f64::NEG_INFINITY);

        for c_node in &self.c_graph.borrow().c_nodes {
            let hitbox = c_node.borrow().hitbox;
            top_left.x = top_left.x.min(hitbox.x);
            top_left.y = top_left.y.min(hitbox.y);
            bottom_right.x = bottom_right.x.max(hitbox.x + hitbox.width);
            bottom_right.y = bottom_right.y.max(hitbox.y + hitbox.height);
        }

        layered_graph.offset().reset();
        layered_graph.offset().add_values(-top_left.x, -top_left.y);
        layered_graph
            .size()
            .set_values(bottom_right.x - top_left.x, bottom_right.y - top_left.y);

        self.apply_external_port_positions(top_left, bottom_right);
    }

    fn apply_external_port_positions(&self, top_left: KVector, bottom_right: KVector) {
        for origin in self.cnode_origin.values() {
            let CNodeOrigin::Node(node) = origin else {
                continue;
            };
            if let Ok(mut node_guard) = node.lock() {
                if node_guard.node_type() != NodeType::ExternalPort {
                    continue;
                }
                let side = node_guard
                    .get_property(InternalProperties::EXT_PORT_SIDE)
                    .unwrap_or(PortSide::Undefined);
                let margin = node_guard.margin().clone();
                let size = *node_guard.shape().size_ref();
                match side {
                    PortSide::West => {
                        node_guard.shape().position().x = top_left.x;
                    }
                    PortSide::East => {
                        node_guard.shape().position().x = bottom_right.x - (size.x + margin.right);
                    }
                    PortSide::North => {
                        node_guard.shape().position().y = top_left.y;
                    }
                    PortSide::South => {
                        node_guard.shape().position().y = bottom_right.y - (size.y + margin.bottom);
                    }
                    PortSide::Undefined => {}
                }
            }
        }
    }

    fn collect_vertical_segments_orthogonal(&mut self) -> Vec<VerticalSegment> {
        let mut segments = Vec::new();

        for node in self.layer_nodes() {
            let Some(c_node) = self.node_to_cnode.get(&arc_key(&node)).cloned() else {
                continue;
            };

            let (outgoing, incoming, node_hitbox) = if let Ok(node_guard) = node.lock() {
                (
                    node_guard.outgoing_edges(),
                    node_guard.incoming_edges(),
                    c_node.borrow().hitbox,
                )
            } else {
                continue;
            };

            for edge in outgoing {
                let (source_side, bend_points) = if let Ok(edge_guard) = edge.lock() {
                    let source_side = edge_guard
                        .source()
                        .and_then(|source| source.lock().ok().map(|port| port.side()))
                        .unwrap_or(PortSide::Undefined);
                    (source_side, edge_guard.bend_points_ref().to_array())
                } else {
                    continue;
                };

                if bend_points.is_empty() {
                    continue;
                }

                let first_bend = bend_points[0];
                match source_side {
                    PortSide::North => {
                        segments.push(self.new_vertical_segment(
                            first_bend,
                            KVector::with_values(first_bend.x, node_hitbox.y),
                            &edge,
                            vec![0],
                            Some(c_node.clone()),
                        ));
                    }
                    PortSide::South => {
                        segments.push(self.new_vertical_segment(
                            first_bend,
                            KVector::with_values(first_bend.x, node_hitbox.y + node_hitbox.height),
                            &edge,
                            vec![0],
                            Some(c_node.clone()),
                        ));
                    }
                    _ => {}
                }

                let mut bend1 = first_bend;

                for (i, bend2) in bend_points.iter().enumerate().skip(1) {
                    let bend2 = *bend2;
                    if !compare_fuzzy::eq(bend1.y, bend2.y) {
                        let segment =
                            self.new_vertical_segment(bend1, bend2, &edge, vec![i - 1, i], None);
                        segments.push(segment);
                    }
                    bend1 = bend2;
                }
            }

            for edge in incoming {
                let (target_side, bend_points) = if let Ok(edge_guard) = edge.lock() {
                    let target_side = edge_guard
                        .target()
                        .and_then(|target| target.lock().ok().map(|port| port.side()))
                        .unwrap_or(PortSide::Undefined);
                    (target_side, edge_guard.bend_points_ref().to_array())
                } else {
                    continue;
                };
                if bend_points.is_empty() {
                    continue;
                }
                let last_idx = bend_points.len() - 1;
                let bend = bend_points[last_idx];
                match target_side {
                    PortSide::North => {
                        segments.push(self.new_vertical_segment(
                            bend,
                            KVector::with_values(bend.x, node_hitbox.y),
                            &edge,
                            vec![last_idx],
                            Some(c_node.clone()),
                        ));
                    }
                    PortSide::South => {
                        segments.push(self.new_vertical_segment(
                            bend,
                            KVector::with_values(bend.x, node_hitbox.y + node_hitbox.height),
                            &edge,
                            vec![last_idx],
                            Some(c_node.clone()),
                        ));
                    }
                    _ => {}
                }
            }
        }

        segments
    }

    fn collect_vertical_segments_splines(&mut self) -> Vec<VerticalSegment> {
        let mut segments: Vec<VerticalSegment> = Vec::new();
        let mut segment_id_to_index: HashMap<usize, usize> = HashMap::new();

        for node in self.layer_nodes() {
            let outgoing = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing {
                let spline_chain = edge.lock().ok().and_then(|mut edge_guard| {
                    edge_guard.get_property(InternalProperties::SPLINE_ROUTE_START)
                });
                let Some(spline_chain) = spline_chain else {
                    continue;
                };

                let mut last_non_straight_segment_id: Option<usize> = None;
                for spline_segment in spline_chain {
                    let Some(segment) = self.new_spline_vertical_segment(&spline_segment) else {
                        continue;
                    };
                    let current_segment_id = segment.segment_id;
                    if let Some(last_segment_id) = last_non_straight_segment_id {
                        if let Some(index) = segment_id_to_index.get(&last_segment_id).copied() {
                            segments[index].constraint_target_ids.insert(current_segment_id);
                        }
                    }

                    segment_id_to_index.insert(current_segment_id, segments.len());
                    last_non_straight_segment_id = Some(current_segment_id);
                    segments.push(segment);
                }
            }
        }

        segments
    }

    fn merge_segments(&self, mut segments: Vec<VerticalSegment>) -> Vec<VerticalSegment> {
        if segments.is_empty() {
            return Vec::new();
        }

        let mut merged = Vec::new();
        let mut survivor = segments.remove(0);
        for next in segments {
            if segments_intersect(&survivor, &next) {
                survivor = join_segments(survivor, next);
            } else {
                merged.push(survivor);
                survivor = next;
            }
        }
        merged.push(survivor);
        merged
    }

    fn new_vertical_segment(
        &mut self,
        bend1: KVector,
        bend2: KVector,
        edge: &LEdgeRef,
        bend_indices: Vec<usize>,
        potential_parent: Option<CNodeRef>,
    ) -> VerticalSegment {
        let segment_id = self.allocate_segment_id();
        let mut represented_edge_keys = HashSet::new();
        represented_edge_keys.insert(arc_key(edge));

        let affected_bends = bend_indices
            .into_iter()
            .map(|index| (edge.clone(), index))
            .collect::<Vec<_>>();

        let mut potential_group_parents = Vec::new();
        if let Some(parent) = potential_parent {
            potential_group_parents.push(parent);
        }

        VerticalSegment {
            segment_id,
            joined_segment_ids: HashSet::from([segment_id]),
            constraint_target_ids: HashSet::new(),
            hitbox: ElkRectangle::with_values(
                bend1.x.min(bend2.x),
                bend1.y.min(bend2.y),
                (bend1.x - bend2.x).abs(),
                (bend1.y - bend2.y).abs(),
            ),
            represented_edges: vec![edge.clone()],
            represented_edge_keys,
            affected_bends,
            affected_spline_segments: Vec::new(),
            potential_group_parents,
        }
    }

    fn new_spline_vertical_segment(
        &mut self,
        spline_segment: &SplineSegmentRef,
    ) -> Option<VerticalSegment> {
        let (is_straight, hitbox, representative_edge) = spline_segment
            .lock()
            .ok()
            .and_then(|segment| {
                let edge = segment.edges.first().cloned()?;
                Some((segment.is_straight, segment.bounding_box, edge))
            })?;
        if is_straight {
            return None;
        }

        let segment_id = self.allocate_segment_id();
        let mut represented_edge_keys = HashSet::new();
        represented_edge_keys.insert(arc_key(&representative_edge));

        Some(VerticalSegment {
            segment_id,
            joined_segment_ids: HashSet::from([segment_id]),
            constraint_target_ids: HashSet::new(),
            hitbox,
            represented_edges: vec![representative_edge],
            represented_edge_keys,
            affected_bends: Vec::new(),
            affected_spline_segments: vec![spline_segment.clone()],
            potential_group_parents: Vec::new(),
        })
    }

    fn allocate_segment_id(&mut self) -> usize {
        let id = self.next_segment_id;
        self.next_segment_id += 1;
        id
    }

    fn layer_nodes(&self) -> Vec<LNodeRef> {
        self.nodes.clone()
    }

    fn capture_comment_offset_if_needed(&mut self, node: &LNodeRef) -> bool {
        let (is_comment, connected_edges) = if let Ok(mut node_guard) = node.lock() {
            (
                node_guard
                    .get_property(LayeredOptions::COMMENT_BOX)
                    .unwrap_or(false),
                node_guard.connected_edges(),
            )
        } else {
            return false;
        };

        if !is_comment || connected_edges.is_empty() {
            return false;
        }

        let edge = connected_edges[0].clone();
        let other = if let Ok(edge_guard) = edge.lock() {
            let source_node = edge_guard
                .source()
                .and_then(|source| source.lock().ok().and_then(|port| port.node()));
            let target_node = edge_guard
                .target()
                .and_then(|target| target.lock().ok().and_then(|port| port.node()));
            match (source_node, target_node) {
                (Some(source), Some(target)) if Arc::ptr_eq(&source, node) => Some(target),
                (Some(source), Some(target)) if Arc::ptr_eq(&target, node) => Some(source),
                (Some(source), _) => Some(source),
                (_, Some(target)) => Some(target),
                _ => None,
            }
        } else {
            None
        };

        let Some(other) = other else {
            return false;
        };

        let comment_pos = node
            .lock()
            .ok()
            .map(|mut node_guard| *node_guard.shape().position_ref());
        let other_pos = other
            .lock()
            .ok()
            .map(|mut node_guard| *node_guard.shape().position_ref());
        if let (Some(comment_pos), Some(other_pos)) = (comment_pos, other_pos) {
            self.comment_offsets.push(CommentOffset {
                comment: node.clone(),
                anchor: other,
                offset: KVector::with_values(
                    comment_pos.x - other_pos.x,
                    comment_pos.y - other_pos.y,
                ),
            });
            true
        } else {
            false
        }
    }
}

struct SpecialSpacingsHandler {
    node_origins: HashMap<usize, LNodeRef>,
    segment_edge_keys: HashMap<usize, HashSet<usize>>,
    spacings: Option<Spacings>,
}

impl SpecialSpacingsHandler {
    fn new(
        origins: &HashMap<usize, CNodeOrigin>,
        segments: &[VerticalSegment],
        spacings: Option<Spacings>,
    ) -> Self {
        let mut node_origins = HashMap::new();
        let mut segment_edge_keys = HashMap::new();

        for (key, origin) in origins {
            match origin {
                CNodeOrigin::Node(node) => {
                    node_origins.insert(*key, node.clone());
                }
                CNodeOrigin::Segment(index) => {
                    if let Some(segment) = segments.get(*index) {
                        segment_edge_keys.insert(*key, segment.represented_edge_keys.clone());
                    }
                }
            }
        }

        Self {
            node_origins,
            segment_edge_keys,
            spacings,
        }
    }

    fn node_type_or_long_edge(&self, c_node: &CNodeRef) -> NodeType {
        let key = rc_key(c_node);
        if let Some(node) = self.node_origins.get(&key) {
            node.lock()
                .ok()
                .map(|node_guard| node_guard.node_type())
                .unwrap_or(NodeType::Normal)
        } else {
            NodeType::LongEdge
        }
    }

    fn is_external_port_node(&self, c_node: &CNodeRef) -> bool {
        let key = rc_key(c_node);
        self.node_origins.get(&key).is_some_and(|node| {
            node.lock()
                .ok()
                .is_some_and(|node_guard| node_guard.node_type() == NodeType::ExternalPort)
        })
    }

    fn segments_share_edge(&self, c_node1: &CNodeRef, c_node2: &CNodeRef) -> bool {
        let key1 = rc_key(c_node1);
        let key2 = rc_key(c_node2);
        let Some(edges1) = self.segment_edge_keys.get(&key1) else {
            return false;
        };
        let Some(edges2) = self.segment_edge_keys.get(&key2) else {
            return false;
        };
        edges1.iter().any(|edge| edges2.contains(edge))
    }
}

impl ISpacingsHandler for SpecialSpacingsHandler {
    fn get_horizontal_spacing(&self, c_node1: &CNodeRef, c_node2: &CNodeRef) -> f64 {
        if self.segments_share_edge(c_node1, c_node2) {
            return 0.0;
        }
        if self.is_external_port_node(c_node1) || self.is_external_port_node(c_node2) {
            return 0.0;
        }

        self.spacings
            .as_ref()
            .map(|spacings| {
                spacings.get_horizontal_spacing_for_types(
                    self.node_type_or_long_edge(c_node1),
                    self.node_type_or_long_edge(c_node2),
                )
            })
            .unwrap_or(0.0)
    }

    fn get_vertical_spacing(&self, c_node1: &CNodeRef, c_node2: &CNodeRef) -> f64 {
        if self.segments_share_edge(c_node1, c_node2) {
            return 1.0;
        }
        if self.is_external_port_node(c_node1) || self.is_external_port_node(c_node2) {
            return 0.0;
        }

        self.spacings
            .as_ref()
            .map(|spacings| {
                spacings.get_vertical_spacing_for_types(
                    self.node_type_or_long_edge(c_node1),
                    self.node_type_or_long_edge(c_node2),
                )
            })
            .unwrap_or(0.0)
    }
}

fn segments_intersect(a: &VerticalSegment, b: &VerticalSegment) -> bool {
    compare_fuzzy::eq(a.hitbox.x, b.hitbox.x)
        && !(compare_fuzzy::lt(a.hitbox.y + a.hitbox.height, b.hitbox.y)
            || compare_fuzzy::lt(b.hitbox.y + b.hitbox.height, a.hitbox.y))
}

fn join_segments(mut left: VerticalSegment, right: VerticalSegment) -> VerticalSegment {
    left.joined_segment_ids.extend(right.joined_segment_ids);
    left.constraint_target_ids.extend(right.constraint_target_ids);
    left.represented_edges.extend(right.represented_edges);
    left.represented_edge_keys
        .extend(right.represented_edge_keys);
    left.affected_bends.extend(right.affected_bends);
    left.affected_spline_segments
        .extend(right.affected_spline_segments);
    left.potential_group_parents
        .extend(right.potential_group_parents);

    let x = left.hitbox.x.min(right.hitbox.x);
    let y = left.hitbox.y.min(right.hitbox.y);
    let max_x = (left.hitbox.x + left.hitbox.width).max(right.hitbox.x + right.hitbox.width);
    let max_y = (left.hitbox.y + left.hitbox.height).max(right.hitbox.y + right.hitbox.height);
    left.hitbox.set_rect(x, y, max_x - x, max_y - y);
    left
}

fn fuzzy_cmp(left: f64, right: f64) -> std::cmp::Ordering {
    if compare_fuzzy::eq(left, right) {
        std::cmp::Ordering::Equal
    } else {
        left.partial_cmp(&right)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

fn arc_key<T>(value: &Arc<std::sync::Mutex<T>>) -> usize {
    Arc::as_ptr(value) as usize
}

fn rc_key(value: &CNodeRef) -> usize {
    Rc::as_ptr(value) as usize
}
