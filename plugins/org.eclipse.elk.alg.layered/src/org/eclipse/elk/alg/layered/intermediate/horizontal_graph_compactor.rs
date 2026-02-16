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

#[derive(Default)]
pub struct HorizontalGraphCompactor;

impl ILayoutProcessor<LGraph> for HorizontalGraphCompactor {
    fn process(&mut self, layered_graph: &mut LGraph, progress_monitor: &mut dyn IElkProgressMonitor) {
        let strategy = layered_graph
            .get_property(LayeredOptions::COMPACTION_POST_COMPACTION_STRATEGY)
            .unwrap_or(GraphCompactionStrategy::None);
        if strategy == GraphCompactionStrategy::None {
            return;
        }

        let edge_routing = layered_graph
            .get_property(LayeredOptions::EDGE_ROUTING)
            .unwrap_or(EdgeRouting::Undefined);
        if edge_routing != EdgeRouting::Orthogonal {
            // Keep pre-existing behavior for non-orthogonal routes until full Java compactor parity is ported.
            return;
        }

        progress_monitor.begin("Horizontal Compaction", 1.0);

        let spacings = layered_graph.get_property(InternalProperties::SPACINGS);
        let constraint_strategy = layered_graph
            .get_property(LayeredOptions::COMPACTION_POST_COMPACTION_CONSTRAINTS)
            .unwrap_or(ConstraintCalculationStrategy::Scanline);

        let mut context = CompactionContext::new(layered_graph);
        context.transform_nodes();
        context.transform_edges_orthogonal();
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
    hitbox: ElkRectangle,
    represented_edges: Vec<LEdgeRef>,
    represented_edge_keys: HashSet<usize>,
    affected_bends: Vec<(LEdgeRef, usize)>,
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
    c_graph: CGraphRef,
    nodes: Vec<LNodeRef>,
    node_to_cnode: HashMap<usize, CNodeRef>,
    cnode_origin: HashMap<usize, CNodeOrigin>,
    segments: Vec<VerticalSegment>,
    cnode_to_segment: HashMap<usize, usize>,
    connection_locks: HashMap<usize, DirectionLocks>,
    comment_offsets: Vec<CommentOffset>,
}

impl CompactionContext {
    fn new(graph: &LGraph) -> Self {
        let has_edges = graph.layers().iter().any(|layer| {
            layer
                .lock()
                .ok()
                .is_some_and(|layer| {
                    layer
                        .nodes()
                        .iter()
                        .any(|node| node.lock().ok().is_some_and(|node| !node.connected_edges().is_empty()))
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
            c_graph: CGraph::new(directions),
            nodes,
            node_to_cnode: HashMap::new(),
            cnode_origin: HashMap::new(),
            segments: Vec::new(),
            cnode_to_segment: HashMap::new(),
            connection_locks: HashMap::new(),
            comment_offsets: Vec::new(),
        }
    }

    fn transform_nodes(&mut self) {
        let layers = self.layer_nodes();
        for node in layers {
            if self.capture_comment_offset_if_needed(&node) {
                continue;
            }

            let (hitbox, incoming_count, outgoing_count, node_type) = if let Ok(mut node_guard) = node.lock() {
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
        let mut segments = self.collect_vertical_segments_orthogonal();
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

        let merged = self.merge_segments(segments);
        self.segments = merged;

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
    }

    fn compact(
        &mut self,
        strategy: GraphCompactionStrategy,
        constraint_strategy: ConstraintCalculationStrategy,
        spacings: Option<Spacings>,
    ) {
        let mut compactor = OneDimensionalCompactor::new(self.c_graph.clone());
        let spacing_handler = SpecialSpacingsHandler::new(&self.cnode_origin, &self.segments, spacings);
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
            if let (Some(anchor_position), Ok(mut comment)) = (anchor_position, item.comment.lock()) {
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
        }
    }

    fn apply_self_loop_label_offsets(&self) {
        for (node_key, c_node) in &self.node_to_cnode {
            let Some(CNodeOrigin::Node(node)) = self
                .cnode_origin
                .get(&rc_key(c_node))
            else {
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
                let is_self_loop = edge.lock().ok().is_some_and(|edge_guard| edge_guard.is_self_loop());
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
                        node_guard.shape().position().x =
                            bottom_right.x - (size.x + margin.right);
                    }
                    PortSide::North => {
                        node_guard.shape().position().y = top_left.y;
                    }
                    PortSide::South => {
                        node_guard.shape().position().y =
                            bottom_right.y - (size.y + margin.bottom);
                    }
                    PortSide::Undefined => {}
                }
            }
        }
    }

    fn collect_vertical_segments_orthogonal(&self) -> Vec<VerticalSegment> {
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
                    (
                        source_side,
                        edge_guard.bend_points_ref().to_array(),
                    )
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
        &self,
        bend1: KVector,
        bend2: KVector,
        edge: &LEdgeRef,
        bend_indices: Vec<usize>,
        potential_parent: Option<CNodeRef>,
    ) -> VerticalSegment {
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
            hitbox: ElkRectangle::with_values(
                bend1.x.min(bend2.x),
                bend1.y.min(bend2.y),
                (bend1.x - bend2.x).abs(),
                (bend1.y - bend2.y).abs(),
            ),
            represented_edges: vec![edge.clone()],
            represented_edge_keys,
            affected_bends,
            potential_group_parents,
        }
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

        let comment_pos = node.lock().ok().map(|mut node_guard| *node_guard.shape().position_ref());
        let other_pos = other
            .lock()
            .ok()
            .map(|mut node_guard| *node_guard.shape().position_ref());
        if let (Some(comment_pos), Some(other_pos)) = (comment_pos, other_pos) {
            self.comment_offsets.push(CommentOffset {
                comment: node.clone(),
                anchor: other,
                offset: KVector::with_values(comment_pos.x - other_pos.x, comment_pos.y - other_pos.y),
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
    left.represented_edges.extend(right.represented_edges);
    left.represented_edge_keys
        .extend(right.represented_edge_keys);
    left.affected_bends.extend(right.affected_bends);
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
        left.partial_cmp(&right).unwrap_or(std::cmp::Ordering::Equal)
    }
}

fn arc_key<T>(value: &Arc<std::sync::Mutex<T>>) -> usize {
    Arc::as_ptr(value) as usize
}

fn rc_key(value: &CNodeRef) -> usize {
    Rc::as_ptr(value) as usize
}
