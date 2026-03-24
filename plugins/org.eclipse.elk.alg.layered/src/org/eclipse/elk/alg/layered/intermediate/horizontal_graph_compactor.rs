use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::compaction::oned::compare_fuzzy;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::compaction::oned::{
    CGraph, CGraphRef, CGroup, CNode, CNodeRef, ICompactionAlgorithm,
    IConstraintCalculationAlgorithm, ISpacingsHandler, OneDimensionalCompactor,
    QuadraticConstraintCalculation,
};
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::compaction::Scanline;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::networksimplex::{
    NEdge, NGraph, NNode, NNodeRef, NetworkSimplex,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkRectangle, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{Direction, EdgeRouting, PortSide};
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraph, LNodeRef, LPortRef, NodeType,
};
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
        context.compact(layered_graph, strategy, constraint_strategy, spacings);
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
    ignore_spacing_up: bool,
    ignore_spacing_down: bool,
    port: Option<LPortRef>,
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
            let layer = layer.lock();
            layer.nodes().iter().any(|node| {
                let node = node.lock();
                !node.connected_edges().is_empty()
            })
        });

        let mut directions = vec![Direction::Left, Direction::Right];
        if !has_edges {
            directions.extend([Direction::Up, Direction::Down]);
        }

        let mut nodes = Vec::new();
        for layer in graph.layers() {
            let layer_guard = layer.lock();
            nodes.extend(layer_guard.nodes().iter().cloned());
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

            let (hitbox, incoming_count, outgoing_count, node_type) = {
                let mut node_guard = node.lock();
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
                let edge_guard = edge.lock();
                if let Some(source) = edge_guard.source() {
                    incoming_ports.insert(arc_key(&source));
                }
                if let Some(target) = edge_guard.target() {
                    outgoing_ports.insert(arc_key(&target));
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
        layered_graph: &LGraph,
        strategy: GraphCompactionStrategy,
        constraint_strategy: ConstraintCalculationStrategy,
        spacings: Option<Spacings>,
    ) {
        let mut compactor = OneDimensionalCompactor::new(self.c_graph.clone());
        let metadata =
            CompactionMetadata::from_context(&self.cnode_origin, &self.segments, layered_graph);
        let vertical_edge_edge_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_EDGE)
            .unwrap_or(0.0);
        let spacing_handler =
            SpecialSpacingsHandler::new(&self.cnode_origin, &self.segments, spacings);
        compactor.set_spacings_handler(Box::new(spacing_handler));

        match constraint_strategy {
            ConstraintCalculationStrategy::Quadratic => {
                compactor.set_constraint_algorithm(Box::new(QuadraticConstraintCalculation));
            }
            ConstraintCalculationStrategy::Scanline => {
                compactor.set_constraint_algorithm(Box::new(
                    EdgeAwareScanlineConstraintCalculation::new(
                        self.edge_routing,
                        vertical_edge_edge_spacing,
                        metadata.clone(),
                    ),
                ));
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
                compactor.set_compaction_algorithm(Box::new(NetworkSimplexCompaction::new(
                    metadata.clone(),
                )));
                compactor.compact();
            }
        }

        compactor.finish();
    }

    fn apply_layout(&self, layered_graph: &mut LGraph) {
        for c_node in &self.c_graph.borrow().c_nodes {
            let key = rc_key(c_node);
            if let Some(CNodeOrigin::Node(node)) = self.cnode_origin.get(&key) {
                let mut node_guard = node.lock();
                let left_margin = node_guard.margin().left;
                node_guard.shape().position().x = c_node.borrow().hitbox.x + left_margin;
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
            let anchor_position = {
                let mut anchor = item.anchor.lock();
                *anchor.shape().position_ref()
            };
            {
                let mut comment = item.comment.lock();
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
                {
                    let mut edge_guard = edge.lock();
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
                {
                    let mut segment_guard = spline_segment.lock();
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

            let outgoing = {
                let node_guard = node.lock();
                node_guard.outgoing_edges()
            };
            for edge in outgoing {
                let is_self_loop = {
                    let edge_guard = edge.lock();
                    edge_guard.is_self_loop()
                };
                if !is_self_loop {
                    continue;
                }
                {
                    let mut edge_guard = edge.lock();
                    edge_guard.bend_points().offset(delta_x, 0.0);
                }
            }
        }
    }

    fn adjust_straight_spline_segments(&self) {
        for node in self.layer_nodes() {
            let outgoing = {
                let node_guard = node.lock();
                node_guard.outgoing_edges()
            };
            for edge in outgoing {
                let spline = {
                    let edge_guard = edge.lock();
                    edge_guard.get_property(InternalProperties::SPLINE_ROUTE_START)
                };
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
            let process = {
                let segment = last_segment.lock();
                segment.initial_segment || !segment.is_straight
            };
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
            let is_match = {
                let guard = segment.lock();
                i == spline.len() - 1 || !guard.is_straight
            };
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
        let (left_initial, left_straight, left_bbox_x, left_bbox_w, left_source_node) = {
            let segment = left.lock();
            (
                segment.initial_segment,
                segment.is_straight,
                segment.bounding_box.x,
                segment.bounding_box.width,
                segment.source_node.clone(),
            )
        };

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

        let (right_last, right_straight, right_bbox_x, right_target_node) = {
            let segment = right.lock();
            (
                segment.last_segment,
                segment.is_straight,
                segment.bounding_box.x,
                segment.target_node.clone(),
            )
        };

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
                {
                    let mut segment = segment_ref.lock();
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

            let outgoing = {
                let node_guard = node.lock();
                node_guard.outgoing_edges()
            };
            for edge in outgoing {
                let is_self_loop = {
                    let edge_guard = edge.lock();
                    edge_guard.is_self_loop()
                };
                if !is_self_loop {
                    continue;
                }
                let labels = {
                    let edge_guard = edge.lock();
                    edge_guard.labels().clone()
                };
                for label in labels {
                    let mut label_guard = label.lock();
                    label_guard.shape().position().x += delta_x;
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
            let mut node_guard = node.lock();
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

    fn collect_vertical_segments_orthogonal(&mut self) -> Vec<VerticalSegment> {
        let mut segments = Vec::new();

        for node in self.layer_nodes() {
            let Some(c_node) = self.node_to_cnode.get(&arc_key(&node)).cloned() else {
                continue;
            };

            let (outgoing, incoming, node_hitbox) = {
                let node_guard = node.lock();
                (
                    node_guard.outgoing_edges(),
                    node_guard.incoming_edges(),
                    c_node.borrow().hitbox,
                )
            };

            for edge in outgoing {
                let (source_port, target_port, bend_points) = {
                    let edge_guard = edge.lock();
                    (
                        edge_guard.source(),
                        edge_guard.target(),
                        edge_guard.bend_points_ref().to_array(),
                    )
                };

                if bend_points.is_empty() {
                    continue;
                }

                let source_side = source_port
                    .as_ref()
                    .map(|source| source.lock().side())
                    .unwrap_or(PortSide::Undefined);
                let first_bend = bend_points[0];
                match source_side {
                    PortSide::North => {
                        let mut segment = self.new_vertical_segment(
                            first_bend,
                            KVector::with_values(first_bend.x, node_hitbox.y),
                            &edge,
                            vec![0],
                            Some(c_node.clone()),
                        );
                        segment.ignore_spacing_down = true;
                        segment.port = source_port.clone();
                        segments.push(segment);
                    }
                    PortSide::South => {
                        let mut segment = self.new_vertical_segment(
                            first_bend,
                            KVector::with_values(first_bend.x, node_hitbox.y + node_hitbox.height),
                            &edge,
                            vec![0],
                            Some(c_node.clone()),
                        );
                        segment.ignore_spacing_up = true;
                        segment.port = source_port.clone();
                        segments.push(segment);
                    }
                    _ => {}
                }

                let mut bend1 = first_bend;
                let mut first_regular_segment = true;
                let mut last_regular_segment_index: Option<usize> = None;

                for (i, bend2) in bend_points.iter().enumerate().skip(1) {
                    let bend2 = *bend2;
                    if !compare_fuzzy::eq(bend1.y, bend2.y) {
                        let mut segment =
                            self.new_vertical_segment(bend1, bend2, &edge, vec![i - 1, i], None);
                        if first_regular_segment {
                            first_regular_segment = false;
                            if bend2.y < node_hitbox.y {
                                segment.ignore_spacing_down = true;
                            } else if bend2.y > node_hitbox.y + node_hitbox.height {
                                segment.ignore_spacing_up = true;
                            } else {
                                segment.ignore_spacing_up = true;
                                segment.ignore_spacing_down = true;
                            }
                        }
                        segments.push(segment);
                        last_regular_segment_index = Some(segments.len() - 1);
                    }
                    bend1 = bend2;
                }

                if let Some(index) = last_regular_segment_index {
                    let target_hitbox = target_port
                        .as_ref()
                        .and_then(|target| {
                            let port = target.lock();
                            port.node()
                        })
                        .and_then(|target_node| {
                            self.node_to_cnode.get(&arc_key(&target_node)).cloned()
                        })
                        .map(|target_c_node| target_c_node.borrow().hitbox);
                    if let Some(target_hitbox) = target_hitbox {
                        if bend1.y < target_hitbox.y {
                            segments[index].ignore_spacing_down = true;
                        } else if bend1.y > target_hitbox.y + target_hitbox.height {
                            segments[index].ignore_spacing_up = true;
                        } else {
                            segments[index].ignore_spacing_up = true;
                            segments[index].ignore_spacing_down = true;
                        }
                    }
                }
            }

            for edge in incoming {
                let (target_port, bend_points) = {
                    let edge_guard = edge.lock();
                    (edge_guard.target(), edge_guard.bend_points_ref().to_array())
                };
                if bend_points.is_empty() {
                    continue;
                }
                let target_side = target_port
                    .as_ref()
                    .map(|target| target.lock().side())
                    .unwrap_or(PortSide::Undefined);
                let last_idx = bend_points.len() - 1;
                let bend = bend_points[last_idx];
                match target_side {
                    PortSide::North => {
                        let mut segment = self.new_vertical_segment(
                            bend,
                            KVector::with_values(bend.x, node_hitbox.y),
                            &edge,
                            vec![last_idx],
                            Some(c_node.clone()),
                        );
                        segment.ignore_spacing_down = true;
                        segment.port = target_port.clone();
                        segments.push(segment);
                    }
                    PortSide::South => {
                        let mut segment = self.new_vertical_segment(
                            bend,
                            KVector::with_values(bend.x, node_hitbox.y + node_hitbox.height),
                            &edge,
                            vec![last_idx],
                            Some(c_node.clone()),
                        );
                        segment.ignore_spacing_up = true;
                        segment.port = target_port.clone();
                        segments.push(segment);
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
            let outgoing = {
                let node_guard = node.lock();
                node_guard.outgoing_edges()
            };
            for edge in outgoing {
                let spline_chain = {
                    let edge_guard = edge.lock();
                    edge_guard.get_property(InternalProperties::SPLINE_ROUTE_START)
                };
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
                            segments[index]
                                .constraint_target_ids
                                .insert(current_segment_id);
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
            ignore_spacing_up: false,
            ignore_spacing_down: false,
            port: None,
            affected_bends,
            affected_spline_segments: Vec::new(),
            potential_group_parents,
        }
    }

    fn new_spline_vertical_segment(
        &mut self,
        spline_segment: &SplineSegmentRef,
    ) -> Option<VerticalSegment> {
        let (is_straight, hitbox, representative_edge) = {
            let segment = spline_segment.lock();
            let edge = segment.edges.first().cloned()?;
            (segment.is_straight, segment.bounding_box, edge)
        };
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
            ignore_spacing_up: false,
            ignore_spacing_down: false,
            port: None,
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
        let (is_comment, connected_edges) = {
            let node_guard = node.lock();
            (
                node_guard
                    .get_property(LayeredOptions::COMMENT_BOX)
                    .unwrap_or(false),
                node_guard.connected_edges(),
            )
        };

        if !is_comment || connected_edges.is_empty() {
            return false;
        }

        let edge = connected_edges[0].clone();
        let other = {
            let edge_guard = edge.lock();
            let source_node = edge_guard
                .source()
                .map(|source| {
                    let port = source.lock();
                    port.node()
                })
                .flatten();
            let target_node = edge_guard
                .target()
                .map(|target| {
                    let port = target.lock();
                    port.node()
                })
                .flatten();
            match (source_node, target_node) {
                (Some(source), Some(target)) if Arc::ptr_eq(&source, node) => Some(target),
                (Some(source), Some(target)) if Arc::ptr_eq(&target, node) => Some(source),
                (Some(source), _) => Some(source),
                (_, Some(target)) => Some(target),
                _ => None,
            }
        };

        let Some(other) = other else {
            return false;
        };

        let comment_pos = {
            let mut node_guard = node.lock();
            *node_guard.shape().position_ref()
        };
        let other_pos = {
            let mut node_guard = other.lock();
            *node_guard.shape().position_ref()
        };
        self.comment_offsets.push(CommentOffset {
            comment: node.clone(),
            anchor: other,
            offset: KVector::with_values(
                comment_pos.x - other_pos.x,
                comment_pos.y - other_pos.y,
            ),
        });
        true
    }
}

const EDGE_AWARE_EPSILON: f64 = 0.5;
const EDGE_AWARE_SMALL_EPSILON: f64 = 0.01;
const NETWORK_SIMPLEX_SEPARATION_WEIGHT: f64 = 1.0;
const NETWORK_SIMPLEX_EDGE_WEIGHT: f64 = 100.0;

#[derive(Clone)]
struct SegmentCompactionInfo {
    represented_edges: Vec<LEdgeRef>,
    represented_edge_keys: HashSet<usize>,
    ignore_spacing_up: bool,
    ignore_spacing_down: bool,
    port: Option<LPortRef>,
}

#[derive(Clone)]
struct CompactionMetadata {
    node_origins: HashMap<usize, LNodeRef>,
    segment_infos: HashMap<usize, SegmentCompactionInfo>,
    node_spacing_node_node: HashMap<usize, f64>,
    node_spacing_edge_edge: HashMap<usize, f64>,
}

impl CompactionMetadata {
    fn from_context(
        origins: &HashMap<usize, CNodeOrigin>,
        segments: &[VerticalSegment],
        graph: &LGraph,
    ) -> Self {
        let mut node_origins = HashMap::new();
        let mut segment_infos = HashMap::new();
        let mut node_spacing_node_node = HashMap::new();
        let mut node_spacing_edge_edge = HashMap::new();

        for (key, origin) in origins {
            match origin {
                CNodeOrigin::Node(node) => {
                    node_origins.insert(*key, node.clone());
                    node_spacing_node_node.insert(
                        *key,
                        Spacings::get_individual_or_default_with_graph(
                            graph,
                            node,
                            LayeredOptions::SPACING_NODE_NODE,
                        ),
                    );
                    node_spacing_edge_edge.insert(
                        *key,
                        Spacings::get_individual_or_default_with_graph(
                            graph,
                            node,
                            LayeredOptions::SPACING_EDGE_EDGE,
                        ),
                    );
                }
                CNodeOrigin::Segment(index) => {
                    if let Some(segment) = segments.get(*index) {
                        segment_infos.insert(
                            *key,
                            SegmentCompactionInfo {
                                represented_edges: segment.represented_edges.clone(),
                                represented_edge_keys: segment.represented_edge_keys.clone(),
                                ignore_spacing_up: segment.ignore_spacing_up,
                                ignore_spacing_down: segment.ignore_spacing_down,
                                port: segment.port.clone(),
                            },
                        );
                    }
                }
            }
        }

        Self {
            node_origins,
            segment_infos,
            node_spacing_node_node,
            node_spacing_edge_edge,
        }
    }

    fn node_origin(&self, c_node: &CNodeRef) -> Option<&LNodeRef> {
        self.node_origins.get(&rc_key(c_node))
    }

    fn segment_info(&self, c_node: &CNodeRef) -> Option<&SegmentCompactionInfo> {
        self.segment_infos.get(&rc_key(c_node))
    }

    fn is_segment(&self, c_node: &CNodeRef) -> bool {
        self.segment_info(c_node).is_some()
    }

    fn is_external_port_node(&self, c_node: &CNodeRef) -> bool {
        self.node_origin(c_node).is_some_and(|node| {
            let node_guard = node.lock();
            node_guard.node_type() == NodeType::ExternalPort
        })
    }

    fn node_node_spacing(&self, c_node: &CNodeRef) -> Option<f64> {
        self.node_spacing_node_node.get(&rc_key(c_node)).copied()
    }

    fn node_edge_spacing(&self, c_node: &CNodeRef) -> Option<f64> {
        self.node_spacing_edge_edge.get(&rc_key(c_node)).copied()
    }

    fn segment_port(&self, c_node: &CNodeRef) -> Option<LPortRef> {
        self.segment_info(c_node).and_then(|info| info.port.clone())
    }

    fn segments_share_edge(&self, c_node1: &CNodeRef, c_node2: &CNodeRef) -> bool {
        let Some(edges1) = self
            .segment_info(c_node1)
            .map(|info| &info.represented_edge_keys)
        else {
            return false;
        };
        let Some(edges2) = self
            .segment_info(c_node2)
            .map(|info| &info.represented_edge_keys)
        else {
            return false;
        };
        edges1.iter().any(|edge| edges2.contains(edge))
    }
}

struct EdgeAwareScanlineConstraintCalculation {
    edge_routing: EdgeRouting,
    vertical_edge_edge_spacing: f64,
    metadata: CompactionMetadata,
}

impl EdgeAwareScanlineConstraintCalculation {
    fn new(
        edge_routing: EdgeRouting,
        vertical_edge_edge_spacing: f64,
        metadata: CompactionMetadata,
    ) -> Self {
        Self {
            edge_routing,
            vertical_edge_edge_spacing,
            metadata,
        }
    }

    fn calculate_for_spline(&self, compactor: &mut OneDimensionalCompactor) {
        self.sweep(compactor, |node| self.metadata.is_segment(node));

        let min_spacing = self.minimum_spacing(compactor);
        let nodes = compactor.c_graph.borrow().c_nodes.clone();
        for node in &nodes {
            if self.metadata.node_origin(node).is_some() {
                self.alter_hitbox(node, min_spacing, 1.0);
            }
        }

        self.sweep(compactor, |_node| true);

        for node in &nodes {
            if self.metadata.node_origin(node).is_some() {
                self.alter_hitbox(node, min_spacing, -1.0);
            }
        }
    }

    fn calculate_for_orthogonal(&self, compactor: &mut OneDimensionalCompactor) {
        let segment_spacing = self.segment_spacing();
        let nodes = compactor.c_graph.borrow().c_nodes.clone();

        let mut segment_nodes = Vec::new();
        for node in &nodes {
            if self.metadata.is_segment(node) {
                self.alter_hitbox(node, segment_spacing, 1.0);
                segment_nodes.push(node.clone());
            }
        }
        self.sweep(compactor, |node| self.metadata.is_segment(node));
        for node in segment_nodes {
            self.alter_hitbox(&node, segment_spacing, -1.0);
        }

        let mut node_spacing_schedule = Vec::new();
        for node in &nodes {
            if self.metadata.node_origin(node).is_some() {
                let spacing = self
                    .metadata
                    .node_edge_spacing(node)
                    .unwrap_or(self.vertical_edge_edge_spacing);
                let adjusted = (spacing / 2.0 - EDGE_AWARE_EPSILON).max(0.0);
                self.alter_hitbox(node, adjusted, 1.0);
                node_spacing_schedule.push((node.clone(), adjusted));
            }
        }
        self.sweep(compactor, |node| self.metadata.node_origin(node).is_some());
        for (node, spacing) in node_spacing_schedule {
            self.alter_hitbox(&node, spacing, -1.0);
        }

        let min_spacing = self.minimum_spacing(compactor);
        let groups = compactor.c_graph.borrow().c_groups.clone();
        for group in &groups {
            self.alter_grouped_hitbox_orthogonal(group, min_spacing, 1.0);
        }
        self.sweep(compactor, |_node| true);
        for group in &groups {
            self.alter_grouped_hitbox_orthogonal(group, min_spacing, -1.0);
        }
    }

    fn segment_spacing(&self) -> f64 {
        (self.vertical_edge_edge_spacing / 2.0 - EDGE_AWARE_EPSILON).max(0.0)
    }

    fn minimum_spacing(&self, compactor: &OneDimensionalCompactor) -> f64 {
        let mut min_spacing = f64::INFINITY;
        let nodes = compactor.c_graph.borrow().c_nodes.clone();
        for node in &nodes {
            if self.metadata.is_external_port_node(node) {
                continue;
            }

            if self.metadata.is_segment(node) {
                min_spacing = min_spacing.min(self.segment_spacing());
                continue;
            }

            if let Some(spacing) = self.metadata.node_node_spacing(node) {
                min_spacing = min_spacing.min((spacing / 2.0 - EDGE_AWARE_EPSILON).max(0.0));
            }
        }

        if min_spacing.is_finite() {
            min_spacing
        } else {
            0.0
        }
    }

    fn alter_grouped_hitbox_orthogonal(
        &self,
        group: &Rc<std::cell::RefCell<CGroup>>,
        spacing: f64,
        fac: f64,
    ) {
        let master = group
            .borrow()
            .master
            .clone()
            .or_else(|| group.borrow().c_nodes.first().cloned());
        let Some(master) = master else {
            return;
        };
        self.alter_hitbox(&master, spacing, fac);

        let group_nodes = group.borrow().c_nodes.clone();
        if group_nodes.len() <= 1 {
            return;
        }

        let delta = spacing * fac;
        for node in group_nodes {
            if Rc::ptr_eq(&node, &master) {
                continue;
            }
            let Some(segment) = self.metadata.segment_info(&node) else {
                continue;
            };
            let mut node_mut = node.borrow_mut();
            if segment.ignore_spacing_up {
                node_mut.hitbox.y += delta + EDGE_AWARE_SMALL_EPSILON;
                node_mut.hitbox.height -= delta + EDGE_AWARE_SMALL_EPSILON;
            } else if segment.ignore_spacing_down {
                node_mut.hitbox.height -= delta + EDGE_AWARE_SMALL_EPSILON;
            }
        }
    }

    fn alter_hitbox(&self, node: &CNodeRef, spacing: f64, fac: f64) {
        let delta = spacing * fac;
        let mut node_mut = node.borrow_mut();
        if let Some(segment) = self.metadata.segment_info(node) {
            if !segment.ignore_spacing_up {
                node_mut.hitbox.y -= delta + EDGE_AWARE_SMALL_EPSILON;
                node_mut.hitbox.height += delta + EDGE_AWARE_SMALL_EPSILON;
            } else if !segment.ignore_spacing_down {
                node_mut.hitbox.height += delta + EDGE_AWARE_SMALL_EPSILON;
            }
        } else if self.metadata.node_origin(node).is_some() {
            node_mut.hitbox.y -= delta;
            node_mut.hitbox.height += 2.0 * delta;
        }
    }

    fn sweep<F>(&self, compactor: &mut OneDimensionalCompactor, filter: F)
    where
        F: Fn(&CNodeRef) -> bool,
    {
        let all_nodes = compactor.c_graph.borrow().c_nodes.clone();
        for (index, node) in all_nodes.iter().enumerate() {
            node.borrow_mut().id = index as i32;
        }

        let mut points = Vec::new();
        for node in all_nodes {
            if filter(&node) {
                points.push(EdgeAwareTimestamp {
                    node: node.clone(),
                    low: true,
                });
                points.push(EdgeAwareTimestamp { node, low: false });
            }
        }
        if points.is_empty() {
            return;
        }

        let node_count = compactor.c_graph.borrow().c_nodes.len();
        let mut handler = EdgeAwareScanlineHandler::new(node_count);
        Scanline::execute(
            points,
            edge_aware_timestamp_cmp,
            &mut |timestamp: &EdgeAwareTimestamp| {
                handler.handle(timestamp);
            },
        );
    }
}

impl IConstraintCalculationAlgorithm for EdgeAwareScanlineConstraintCalculation {
    fn calculate_constraints(&self, compactor: &mut OneDimensionalCompactor) {
        match self.edge_routing {
            EdgeRouting::Orthogonal => self.calculate_for_orthogonal(compactor),
            EdgeRouting::Splines => self.calculate_for_spline(compactor),
            _ => {}
        }
    }
}

#[derive(Clone)]
struct EdgeAwareTimestamp {
    node: CNodeRef,
    low: bool,
}

fn edge_aware_timestamp_cmp(left: &EdgeAwareTimestamp, right: &EdgeAwareTimestamp) -> Ordering {
    let mut left_y = left.node.borrow().hitbox.y;
    if !left.low {
        left_y += left.node.borrow().hitbox.height;
    }

    let mut right_y = right.node.borrow().hitbox.y;
    if !right.low {
        right_y += right.node.borrow().hitbox.height;
    }

    let cmp = left_y.partial_cmp(&right_y).unwrap_or(Ordering::Equal);
    if cmp == Ordering::Equal {
        if !left.low && right.low {
            return Ordering::Less;
        }
        if !right.low && left.low {
            return Ordering::Greater;
        }
    }
    cmp
}

struct EdgeAwareScanlineHandler {
    intervals: Vec<CNodeRef>,
    cand: Vec<Option<CNodeRef>>,
}

impl EdgeAwareScanlineHandler {
    fn new(node_count: usize) -> Self {
        Self {
            intervals: Vec::new(),
            cand: vec![None; node_count],
        }
    }

    fn handle(&mut self, timestamp: &EdgeAwareTimestamp) {
        if timestamp.low {
            self.insert(&timestamp.node);
        } else {
            self.delete(&timestamp.node);
        }
    }

    fn insert(&mut self, node: &CNodeRef) {
        let position = self.insertion_pos(node);
        self.intervals.insert(position, node.clone());

        let node_id = node.borrow().id as usize;
        self.cand[node_id] = self.lower(node);

        if let Some(right) = self.higher(node) {
            let right_id = right.borrow().id as usize;
            self.cand[right_id] = Some(node.clone());
        }
    }

    fn delete(&mut self, node: &CNodeRef) {
        if let Some(left) = self.lower(node) {
            let node_id = node.borrow().id as usize;
            if self.cand[node_id]
                .as_ref()
                .is_some_and(|candidate| Rc::ptr_eq(candidate, &left))
                && in_different_groups(&left, node)
            {
                left.borrow_mut().constraints.push(node.clone());
            }
        }

        if let Some(right) = self.higher(node) {
            let right_id = right.borrow().id as usize;
            if self.cand[right_id]
                .as_ref()
                .is_some_and(|candidate| Rc::ptr_eq(candidate, node))
                && in_different_groups(&right, node)
            {
                node.borrow_mut().constraints.push(right.clone());
            }
        }

        self.intervals
            .retain(|candidate| !Rc::ptr_eq(candidate, node));
    }

    fn insertion_pos(&self, node: &CNodeRef) -> usize {
        let center = cnode_center_x(node);
        let mut position = 0usize;
        while position < self.intervals.len() && cnode_center_x(&self.intervals[position]) < center
        {
            position += 1;
        }
        position
    }

    fn index_of(&self, node: &CNodeRef) -> Option<usize> {
        self.intervals
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, node))
    }

    fn lower(&self, node: &CNodeRef) -> Option<CNodeRef> {
        let index = self.index_of(node)?;
        if index > 0 {
            Some(self.intervals[index - 1].clone())
        } else {
            None
        }
    }

    fn higher(&self, node: &CNodeRef) -> Option<CNodeRef> {
        let index = self.index_of(node)?;
        if index + 1 < self.intervals.len() {
            Some(self.intervals[index + 1].clone())
        } else {
            None
        }
    }
}

struct NetworkSimplexCompaction {
    metadata: CompactionMetadata,
}

impl NetworkSimplexCompaction {
    fn new(metadata: CompactionMetadata) -> Self {
        Self { metadata }
    }

    fn add_group_edge(
        &self,
        group_to_nnode: &HashMap<usize, NNodeRef>,
        source_group: &Rc<std::cell::RefCell<CGroup>>,
        target_group: &Rc<std::cell::RefCell<CGroup>>,
        delta: i32,
        weight: f64,
    ) {
        let Some(source) = group_to_nnode.get(&group_key(source_group)).cloned() else {
            return;
        };
        let Some(target) = group_to_nnode.get(&group_key(target_group)).cloned() else {
            return;
        };
        self.add_nnode_edge(source, target, delta, weight);
    }

    fn add_nnode_edge(&self, source: NNodeRef, target: NNodeRef, delta: i32, weight: f64) {
        if Arc::ptr_eq(&source, &target) {
            return;
        }
        NEdge::of()
            .delta(delta.max(0))
            .weight(weight)
            .source(source)
            .target(target)
            .create();
    }

    fn add_separation_constraints(
        &self,
        compactor: &mut OneDimensionalCompactor,
        network_simplex_graph: &mut NGraph,
        group_to_nnode: &HashMap<usize, NNodeRef>,
    ) {
        let c_nodes = compactor.c_graph.borrow().c_nodes.clone();
        for c_node in &c_nodes {
            let constraints = c_node.borrow().constraints.clone();
            for inc_node in constraints {
                let Some(source_group) = c_node.borrow().group() else {
                    continue;
                };
                let Some(target_group) = inc_node.borrow().group() else {
                    continue;
                };
                if Rc::ptr_eq(&source_group, &target_group) {
                    continue;
                }

                let spacing = if compactor.direction.is_horizontal() {
                    compactor
                        .spacings_handler
                        .get_horizontal_spacing(c_node, &inc_node)
                } else {
                    compactor
                        .spacings_handler
                        .get_vertical_spacing(c_node, &inc_node)
                };
                let source_offset = c_node.borrow().c_group_offset.x;
                let source_width = c_node.borrow().hitbox.width;
                let target_offset = inc_node.borrow().c_group_offset.x;
                let delta = (source_offset + source_width + spacing - target_offset)
                    .ceil()
                    .max(0.0) as i32;

                if !self.metadata.segments_share_edge(c_node, &inc_node) {
                    let source_is_segment = self.metadata.is_segment(c_node);
                    let target_is_segment = self.metadata.is_segment(&inc_node);
                    let source_is_node = self.metadata.node_origin(c_node).is_some();
                    let target_is_node = self.metadata.node_origin(&inc_node).is_some();
                    let weight = if (source_is_segment && target_is_node)
                        || (target_is_segment && source_is_node)
                    {
                        2.0
                    } else {
                        NETWORK_SIMPLEX_SEPARATION_WEIGHT
                    };
                    self.add_group_edge(
                        group_to_nnode,
                        &source_group,
                        &target_group,
                        delta,
                        weight,
                    );
                    continue;
                }

                let helper = NNode::of().create(network_simplex_graph);
                let offset_delta = (inc_node.borrow().c_group_offset.x
                    - c_node.borrow().c_group_offset.x)
                    .ceil() as i32;

                let mut adjust = offset_delta as f64
                    - (inc_node.borrow().c_group_offset.x - c_node.borrow().c_group_offset.x);
                let mut alter_offset_node = c_node.clone();
                let mut port = self.metadata.segment_port(c_node);
                if port.is_none() {
                    port = self.metadata.segment_port(&inc_node);
                    adjust = -adjust;
                    alter_offset_node = inc_node.clone();
                }
                if let Some(port) = port {
                    alter_offset_node.borrow_mut().c_group_offset.x -= adjust;
                    let mut port_guard = port.lock();
                    port_guard.shape().position().x -= adjust;
                }

                let Some(source_nnode) = group_to_nnode.get(&group_key(&source_group)).cloned()
                else {
                    continue;
                };
                let Some(target_nnode) = group_to_nnode.get(&group_key(&target_group)).cloned()
                else {
                    continue;
                };
                self.add_nnode_edge(
                    helper.clone(),
                    source_nnode,
                    offset_delta.max(0),
                    NETWORK_SIMPLEX_SEPARATION_WEIGHT,
                );
                self.add_nnode_edge(
                    helper,
                    target_nnode,
                    (-offset_delta).max(0),
                    NETWORK_SIMPLEX_SEPARATION_WEIGHT,
                );
            }
        }
    }

    fn add_edge_constraints(
        &self,
        compactor: &mut OneDimensionalCompactor,
        group_to_nnode: &HashMap<usize, NNodeRef>,
    ) {
        let c_nodes = compactor.c_graph.borrow().c_nodes.clone();
        let mut lnode_map: HashMap<usize, CNodeRef> = HashMap::new();
        let mut ledge_map: HashMap<usize, Vec<CNodeRef>> = HashMap::new();

        for c_node in &c_nodes {
            if let Some(node) = self.metadata.node_origin(c_node) {
                lnode_map.insert(arc_key(node), c_node.clone());
            } else if let Some(segment) = self.metadata.segment_info(c_node) {
                let mut seen = HashSet::new();
                for edge in &segment.represented_edges {
                    let key = arc_key(edge);
                    if seen.insert(key) {
                        ledge_map.entry(key).or_default().push(c_node.clone());
                    }
                }
            }
        }

        for c_node in c_nodes {
            let Some(l_node) = self.metadata.node_origin(&c_node).cloned() else {
                continue;
            };
            let outgoing = {
                let node_guard = l_node.lock();
                node_guard.outgoing_edges()
            };
            for l_edge in outgoing {
                let (is_self_loop, source_port, target_port) = {
                    let edge_guard = l_edge.lock();
                    (
                        edge_guard.is_self_loop(),
                        edge_guard.source(),
                        edge_guard.target(),
                    )
                };
                if is_self_loop {
                    continue;
                }
                let (Some(source_port), Some(target_port)) = (source_port, target_port) else {
                    continue;
                };

                let source_side = {
                    let port = source_port.lock();
                    port.side()
                };
                let target_side = {
                    let port = target_port.lock();
                    port.side()
                };
                if is_north_south_side(source_side) && is_north_south_side(target_side) {
                    continue;
                }

                let target_node = {
                    let port = target_port.lock();
                    port.node()
                };
                let Some(target_node) = target_node else {
                    continue;
                };
                let Some(target_c_node) = lnode_map.get(&arc_key(&target_node)).cloned() else {
                    continue;
                };

                let Some(source_group) = c_node.borrow().group() else {
                    continue;
                };
                let Some(target_group) = target_c_node.borrow().group() else {
                    continue;
                };
                self.add_group_edge(
                    group_to_nnode,
                    &source_group,
                    &target_group,
                    0,
                    NETWORK_SIMPLEX_EDGE_WEIGHT,
                );

                if source_side == PortSide::West {
                    let has_outgoing = {
                        let port = source_port.lock();
                        !port.outgoing_edges().is_empty()
                    };
                    if has_outgoing {
                        if let Some(segment_nodes) = ledge_map.get(&arc_key(&l_edge)) {
                            for segment_node in segment_nodes {
                                if segment_node.borrow().hitbox.x < c_node.borrow().hitbox.x {
                                    let Some(segment_group) = segment_node.borrow().group() else {
                                        continue;
                                    };
                                    self.add_group_edge(
                                        group_to_nnode,
                                        &segment_group,
                                        &source_group,
                                        1,
                                        NETWORK_SIMPLEX_EDGE_WEIGHT,
                                    );
                                }
                            }
                        }
                    }
                }

                if target_side == PortSide::East {
                    let has_incoming = {
                        let port = target_port.lock();
                        !port.incoming_edges().is_empty()
                    };
                    if has_incoming {
                        if let Some(segment_nodes) = ledge_map.get(&arc_key(&l_edge)) {
                            for segment_node in segment_nodes {
                                if segment_node.borrow().hitbox.x > c_node.borrow().hitbox.x {
                                    let Some(segment_group) = segment_node.borrow().group() else {
                                        continue;
                                    };
                                    self.add_group_edge(
                                        group_to_nnode,
                                        &source_group,
                                        &segment_group,
                                        1,
                                        NETWORK_SIMPLEX_EDGE_WEIGHT,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn add_artificial_source_node(&self, network_simplex_graph: &mut NGraph, next_id: &mut i32) {
        let mut sources = Vec::new();
        for node in &network_simplex_graph.nodes {
            let node_guard = node.lock();
            if node_guard.incoming_edges().is_empty() {
                sources.push(node.clone());
            }
        }
        if sources.len() <= 1 {
            return;
        }

        let dummy_source = NNode::of().id(*next_id).create(network_simplex_graph);
        *next_id += 1;
        for source in sources {
            self.add_nnode_edge(dummy_source.clone(), source, 1, 0.0);
        }
    }
}

impl ICompactionAlgorithm for NetworkSimplexCompaction {
    fn compact(&self, compactor: &mut OneDimensionalCompactor) {
        let c_groups = compactor.c_graph.borrow().c_groups.clone();
        let mut network_simplex_graph = NGraph::new();
        let mut group_to_nnode: HashMap<usize, NNodeRef> = HashMap::new();
        let mut next_id = 0i32;

        for group in &c_groups {
            group.borrow_mut().id = next_id;
            let n_node = NNode::of().id(next_id).create(&mut network_simplex_graph);
            group_to_nnode.insert(group_key(group), n_node);
            next_id += 1;
        }

        self.add_separation_constraints(compactor, &mut network_simplex_graph, &group_to_nnode);
        self.add_edge_constraints(compactor, &group_to_nnode);
        self.add_artificial_source_node(&mut network_simplex_graph, &mut next_id);

        let mut simplex = NetworkSimplex::for_graph(&mut network_simplex_graph);
        simplex.execute();

        let c_nodes = compactor.c_graph.borrow().c_nodes.clone();
        for c_node in c_nodes {
            let Some(group) = c_node.borrow().group() else {
                continue;
            };
            let Some(n_node) = group_to_nnode.get(&group_key(&group)) else {
                continue;
            };
            let layer = {
                let node = n_node.lock();
                node.layer
            };
            let offset = c_node.borrow().c_group_offset.x;
            c_node.borrow_mut().hitbox.x = layer as f64 + offset;
        }
    }
}

fn cnode_center_x(node: &CNodeRef) -> f64 {
    let node_ref = node.borrow();
    node_ref.hitbox.x + node_ref.hitbox.width / 2.0
}

fn in_different_groups(left: &CNodeRef, right: &CNodeRef) -> bool {
    let left_group = left.borrow().group();
    let right_group = right.borrow().group();
    match (left_group, right_group) {
        (Some(left_group), Some(right_group)) => !Rc::ptr_eq(&left_group, &right_group),
        _ => true,
    }
}

fn is_north_south_side(side: PortSide) -> bool {
    side == PortSide::North || side == PortSide::South
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
            let node_guard = node.lock();
            node_guard.node_type()
        } else {
            NodeType::LongEdge
        }
    }

    fn is_external_port_node(&self, c_node: &CNodeRef) -> bool {
        let key = rc_key(c_node);
        self.node_origins.get(&key).is_some_and(|node| {
            let node_guard = node.lock();
            node_guard.node_type() == NodeType::ExternalPort
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
    left.constraint_target_ids
        .extend(right.constraint_target_ids);
    left.represented_edges.extend(right.represented_edges);
    left.represented_edge_keys
        .extend(right.represented_edge_keys);
    left.ignore_spacing_up |= right.ignore_spacing_up;
    left.ignore_spacing_down |= right.ignore_spacing_down;
    if left.port.is_none() {
        left.port = right.port;
    }
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

fn arc_key<T>(value: &Arc<Mutex<T>>) -> usize {
    Arc::as_ptr(value) as usize
}

fn rc_key(value: &CNodeRef) -> usize {
    Rc::as_ptr(value) as usize
}

fn group_key(value: &Rc<std::cell::RefCell<CGroup>>) -> usize {
    Rc::as_ptr(value) as usize
}
