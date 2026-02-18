use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, LazyLock, Mutex};

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IElkProgressMonitor, Random};

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraph, LGraphUtil, LNode, LPortRef, LayerRef,
};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions, SplineRoutingMode,
};
use crate::org::eclipse::elk::alg::layered::p5edges::polyline_edge_router::PolylineEdgeRouter;
use crate::org::eclipse::elk::alg::layered::p5edges::splines::spline_segment::{
    Dependency, SideToProcess, SplineSegmentRef,
};
use crate::org::eclipse::elk::alg::layered::p5edges::splines::splines_math::SplinesMath;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

static BASELINE_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_after(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::FinalSplineBendpointsCalculator),
        )
        .add_before(
            LayeredPhases::P3NodeOrdering,
            Arc::new(IntermediateProcessorStrategy::InvertedPortProcessor),
        );
    config
});

static SELF_LOOP_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P1CycleBreaking,
            Arc::new(IntermediateProcessorStrategy::SelfLoopPreprocessor),
        )
        .add_after(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::SelfLoopPostprocessor),
        )
        .before(LayeredPhases::P4NodePlacement)
        .add(Arc::new(
            IntermediateProcessorStrategy::SelfLoopPortRestorer,
        ))
        .add(Arc::new(IntermediateProcessorStrategy::SelfLoopRouter));
    config
});

static CENTER_EDGE_LABEL_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P2Layering,
            Arc::new(IntermediateProcessorStrategy::LabelDummyInserter),
        )
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::LabelDummySwitcher),
        )
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::LabelSideSelector),
        )
        .add_after(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::LabelDummyRemover),
        );
    config
});

static NORTH_SOUTH_PORT_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P3NodeOrdering,
            Arc::new(IntermediateProcessorStrategy::NorthSouthPortPreprocessor),
        )
        .add_before(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::NorthSouthPortPostprocessor),
        );
    config
});

static END_EDGE_LABEL_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::LabelSideSelector),
        )
        .add_before(
            LayeredPhases::P4NodePlacement,
            Arc::new(IntermediateProcessorStrategy::EndLabelPreprocessor),
        )
        .add_after(
            LayeredPhases::P5EdgeRouting,
            Arc::new(IntermediateProcessorStrategy::EndLabelPostprocessor),
        );
    config
});

pub struct SplineEdgeRouter {
    edges_remaining_layer: Vec<LEdgeRef>,
    spline_segments_layer: Vec<SplineSegmentRef>,
    left_ports_layer: Vec<LPortRef>,
    right_ports_layer: Vec<LPortRef>,
    self_loops_layer: Vec<LEdgeRef>,
    start_edges: Vec<LEdgeRef>,
    all_spline_segments: Vec<SplineSegmentRef>,
    edge_to_segment_map: HashMap<usize, SplineSegmentRef>,
    successing_edge: HashMap<usize, LEdgeRef>,
}

impl SplineEdgeRouter {
    const MAX_VERTICAL_DIFF_FOR_STRAIGHT: f64 = 0.2;
    pub const SPLINE_DIMENSION: usize = 3;

    pub fn new() -> Self {
        SplineEdgeRouter {
            edges_remaining_layer: Vec::new(),
            spline_segments_layer: Vec::new(),
            left_ports_layer: Vec::new(),
            right_ports_layer: Vec::new(),
            self_loops_layer: Vec::new(),
            start_edges: Vec::new(),
            all_spline_segments: Vec::new(),
            edge_to_segment_map: HashMap::new(),
            successing_edge: HashMap::new(),
        }
    }

    fn create_segments_and_compute_ranking(&mut self, random: &mut Random) {
        let left_ports = self.left_ports_layer.clone();
        let right_ports = self.right_ports_layer.clone();

        self.create_spline_segments_for_hyper_edges(
            &left_ports,
            &right_ports,
            SideToProcess::Left,
            true,
        );
        self.create_spline_segments_for_hyper_edges(
            &left_ports,
            &right_ports,
            SideToProcess::Left,
            false,
        );
        self.create_spline_segments_for_hyper_edges(
            &left_ports,
            &right_ports,
            SideToProcess::Right,
            true,
        );
        self.create_spline_segments_for_hyper_edges(
            &left_ports,
            &right_ports,
            SideToProcess::Right,
            false,
        );

        self.create_spline_segments();

        let mut source_index = 0usize;
        while source_index < self.spline_segments_layer.len() {
            let source = self.spline_segments_layer[source_index].clone();
            let mut target_index = source_index + 1;
            while target_index < self.spline_segments_layer.len() {
                let target = self.spline_segments_layer[target_index].clone();
                self.create_dependency(&source, &target);
                target_index += 1;
            }
            source_index += 1;
        }

        Self::break_cycles(&self.spline_segments_layer, random);
        Self::topological_numbering(&self.spline_segments_layer);
    }

    fn clear_then_fill_mappings(
        &mut self,
        left_layer: Option<&LayerRef>,
        right_layer: Option<&LayerRef>,
    ) {
        self.left_ports_layer.clear();
        self.right_ports_layer.clear();
        self.edges_remaining_layer.clear();
        self.spline_segments_layer.clear();
        self.self_loops_layer.clear();

        if let Some(left_layer) = left_layer {
            let nodes = left_layer
                .lock()
                .ok()
                .map(|layer| layer.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports_by_side(PortSide::East))
                    .unwrap_or_default();
                for port in ports {
                    Self::insert_unique_port(&mut self.left_ports_layer, &port);
                    let outgoing = port
                        .lock()
                        .ok()
                        .map(|port_guard| port_guard.outgoing_edges().clone())
                        .unwrap_or_default();
                    for edge in outgoing {
                        if edge
                            .lock()
                            .ok()
                            .map(|edge_guard| edge_guard.is_self_loop())
                            .unwrap_or(false)
                        {
                            continue;
                        }

                        self.edges_remaining_layer.push(edge.clone());
                        self.find_and_add_successor(&edge);

                        let source_node = edge
                            .lock()
                            .ok()
                            .and_then(|edge_guard| edge_guard.source())
                            .and_then(|port| {
                                port.lock().ok().and_then(|port_guard| port_guard.node())
                            });
                        if source_node
                            .as_ref()
                            .and_then(|node| {
                                node.lock().ok().map(|node_guard| {
                                    Self::is_qualified_as_starting_node(&node_guard)
                                })
                            })
                            .unwrap_or(false)
                        {
                            self.start_edges.push(edge.clone());
                        }

                        let target_port =
                            edge.lock().ok().and_then(|edge_guard| edge_guard.target());
                        let target_layer = target_port
                            .as_ref()
                            .and_then(|port| {
                                port.lock().ok().and_then(|port_guard| port_guard.node())
                            })
                            .and_then(|node| {
                                node.lock().ok().and_then(|node_guard| node_guard.layer())
                            });

                        if let Some(target_layer) = target_layer.as_ref() {
                            if let Some(right_layer) = right_layer {
                                if Arc::ptr_eq(right_layer, target_layer) {
                                    if let Some(target_port) = target_port {
                                        Self::insert_unique_port(
                                            &mut self.right_ports_layer,
                                            &target_port,
                                        );
                                    }
                                    continue;
                                }
                            }
                            if Arc::ptr_eq(left_layer, target_layer) {
                                if let Some(target_port) = target_port {
                                    Self::insert_unique_port(
                                        &mut self.left_ports_layer,
                                        &target_port,
                                    );
                                }
                                continue;
                            }
                        }
                        remove_arc(&mut self.edges_remaining_layer, &edge);
                    }
                }
            }
        }

        if let Some(right_layer) = right_layer {
            let nodes = right_layer
                .lock()
                .ok()
                .map(|layer| layer.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports().clone())
                    .unwrap_or_default();
                for port in ports {
                    let outgoing = port
                        .lock()
                        .ok()
                        .map(|port_guard| port_guard.outgoing_edges().clone())
                        .unwrap_or_default();
                    for edge in outgoing {
                        if edge
                            .lock()
                            .ok()
                            .map(|edge_guard| edge_guard.is_self_loop())
                            .unwrap_or(false)
                        {
                            self.self_loops_layer.push(edge.clone());
                        }
                    }
                }

                let ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports_by_side(PortSide::West))
                    .unwrap_or_default();
                for port in ports {
                    Self::insert_unique_port(&mut self.right_ports_layer, &port);
                    let outgoing = port
                        .lock()
                        .ok()
                        .map(|port_guard| port_guard.outgoing_edges().clone())
                        .unwrap_or_default();
                    for edge in outgoing {
                        if edge
                            .lock()
                            .ok()
                            .map(|edge_guard| edge_guard.is_self_loop())
                            .unwrap_or(false)
                        {
                            continue;
                        }

                        self.edges_remaining_layer.push(edge.clone());
                        self.find_and_add_successor(&edge);

                        let source_node = edge
                            .lock()
                            .ok()
                            .and_then(|edge_guard| edge_guard.source())
                            .and_then(|port| {
                                port.lock().ok().and_then(|port_guard| port_guard.node())
                            });
                        if source_node
                            .as_ref()
                            .and_then(|node| {
                                node.lock().ok().map(|node_guard| {
                                    Self::is_qualified_as_starting_node(&node_guard)
                                })
                            })
                            .unwrap_or(false)
                        {
                            self.start_edges.push(edge.clone());
                        }

                        let target_port =
                            edge.lock().ok().and_then(|edge_guard| edge_guard.target());
                        let target_layer = target_port
                            .as_ref()
                            .and_then(|port| {
                                port.lock().ok().and_then(|port_guard| port_guard.node())
                            })
                            .and_then(|node| {
                                node.lock().ok().and_then(|node_guard| node_guard.layer())
                            });

                        if let Some(target_layer) = target_layer.as_ref() {
                            if Arc::ptr_eq(right_layer, target_layer) {
                                if let Some(target_port) = target_port {
                                    Self::insert_unique_port(
                                        &mut self.right_ports_layer,
                                        &target_port,
                                    );
                                }
                                continue;
                            }
                            if let Some(left_layer) = left_layer {
                                if Arc::ptr_eq(left_layer, target_layer) {
                                    if let Some(target_port) = target_port {
                                        Self::insert_unique_port(
                                            &mut self.left_ports_layer,
                                            &target_port,
                                        );
                                    }
                                    continue;
                                }
                            }
                        }
                        remove_arc(&mut self.edges_remaining_layer, &edge);
                    }
                }
            }
        }
    }

    fn compute_sloppy_spacing(
        &self,
        right_layer: &LayerRef,
        edge_edge_spacing: f64,
        node_node_spacing: f64,
        sloppy_layer_spacing_factor: f64,
    ) -> f64 {
        let mut max_vert_diff: f64 = 0.0;
        let nodes = right_layer
            .lock()
            .ok()
            .map(|layer| layer.nodes().clone())
            .unwrap_or_default();
        for node in nodes {
            let incoming_edges = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.incoming_edges())
                .unwrap_or_default();
            let mut max_curr_input_y_diff: f64 = 0.0;
            for incoming_edge in incoming_edges {
                let (source_pos, target_pos) = {
                    let edge_guard = incoming_edge.lock().ok();
                    let source = edge_guard.as_ref().and_then(|edge| edge.source());
                    let target = edge_guard.as_ref().and_then(|edge| edge.target());
                    let source_pos = source
                        .and_then(|port| {
                            port.lock()
                                .ok()
                                .and_then(|port_guard| port_guard.absolute_anchor())
                        })
                        .map(|anchor| anchor.y)
                        .unwrap_or(0.0);
                    let target_pos = target
                        .and_then(|port| {
                            port.lock()
                                .ok()
                                .and_then(|port_guard| port_guard.absolute_anchor())
                        })
                        .map(|anchor| anchor.y)
                        .unwrap_or(0.0);
                    (source_pos, target_pos)
                };
                max_curr_input_y_diff = max_curr_input_y_diff.max((target_pos - source_pos).abs());
            }
            max_vert_diff = max_vert_diff.max(max_curr_input_y_diff);
        }

        sloppy_layer_spacing_factor
            * (edge_edge_spacing / node_node_spacing).min(1.0)
            * max_vert_diff
    }

    fn find_and_add_successor(&mut self, edge: &LEdgeRef) {
        let target_node = edge
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.target())
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
        let Some(target_node) = target_node else {
            return;
        };
        let is_normal = target_node
            .lock()
            .ok()
            .map(|node_guard| Self::is_normal_node(&node_guard))
            .unwrap_or(false);
        if is_normal {
            return;
        }
        let outgoing_edges = target_node
            .lock()
            .ok()
            .map(|node_guard| node_guard.outgoing_edges())
            .unwrap_or_default();
        if let Some(first) = outgoing_edges.first() {
            self.successing_edge.insert(edge_key(edge), first.clone());
        }
    }

    fn create_spline_segments(&mut self) {
        let edges = self.edges_remaining_layer.clone();
        for edge in edges {
            let source_port = edge.lock().ok().and_then(|edge_guard| edge_guard.source());
            let target_port = edge.lock().ok().and_then(|edge_guard| edge_guard.target());

            let (source_side, target_side, source_port, target_port) =
                match (source_port, target_port) {
                    (Some(source_port), Some(target_port)) => {
                        let source_side = if contains_port(&self.left_ports_layer, &source_port) {
                            SideToProcess::Left
                        } else if contains_port(&self.right_ports_layer, &source_port) {
                            SideToProcess::Right
                        } else {
                            panic!("Source port must be in one of the port sets.");
                        };
                        let target_side = if contains_port(&self.left_ports_layer, &target_port) {
                            SideToProcess::Left
                        } else if contains_port(&self.right_ports_layer, &target_port) {
                            SideToProcess::Right
                        } else {
                            panic!("Target port must be in one of the port sets.");
                        };
                        (source_side, target_side, source_port, target_port)
                    }
                    _ => continue,
                };

            let segment = crate::org::eclipse::elk::alg::layered::p5edges::splines::spline_segment::SplineSegment::new_single_edge(
                &edge,
                source_side,
                target_side,
            );
            self.edge_to_segment_map
                .insert(edge_key(&edge), segment.clone());
            self.spline_segments_layer.push(segment);

            let _ = (source_port, target_port);
        }
    }

    fn insert_unique_port(ports: &mut Vec<LPortRef>, port: &LPortRef) {
        if contains_port(ports, port) {
            return;
        }
        ports.push(port.clone());
    }

    fn create_spline_segments_for_hyper_edges(
        &mut self,
        left_ports: &[LPortRef],
        right_ports: &[LPortRef],
        side_to_process: SideToProcess,
        reversed: bool,
    ) {
        let ports_to_process = match side_to_process {
            SideToProcess::Left => left_ports,
            SideToProcess::Right => right_ports,
        };

        for single_port in ports_to_process {
            let single_port_position = single_port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.absolute_anchor())
                .map(|anchor| anchor.y)
                .unwrap_or(0.0);

            let mut up_edges: Vec<
                org_eclipse_elk_core::org::eclipse::elk::core::util::pair::Pair<
                    SideToProcess,
                    LEdgeRef,
                >,
            > = Vec::new();
            let mut down_edges: Vec<
                org_eclipse_elk_core::org::eclipse::elk::core::util::pair::Pair<
                    SideToProcess,
                    LEdgeRef,
                >,
            > = Vec::new();

            let connected_edges = single_port
                .lock()
                .ok()
                .map(|port_guard| port_guard.connected_edges())
                .unwrap_or_default();
            for edge in connected_edges {
                let reversed_edge = edge
                    .lock()
                    .ok()
                    .and_then(|mut edge_guard| {
                        edge_guard.get_property(InternalProperties::REVERSED)
                    })
                    .unwrap_or(false);
                if reversed_edge != reversed {
                    continue;
                }
                if !contains_edge(&self.edges_remaining_layer, &edge) {
                    continue;
                }

                let target_port = {
                    let edge_guard = edge.lock().ok();
                    let edge_guard = edge_guard.as_ref();
                    let source_port = edge_guard.and_then(|edge| edge.source());
                    let target_port = edge_guard.and_then(|edge| edge.target());
                    if let (Some(source_port), Some(target_port)) = (source_port, target_port) {
                        if Arc::ptr_eq(&target_port, single_port) {
                            source_port
                        } else {
                            target_port
                        }
                    } else {
                        continue;
                    }
                };

                let target_port_position = target_port
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.absolute_anchor())
                    .map(|anchor| anchor.y)
                    .unwrap_or(0.0);

                if Self::is_straight(target_port_position, single_port_position) {
                    continue;
                }

                if target_port_position < single_port_position {
                    let side = if contains_port(left_ports, &target_port) {
                        SideToProcess::Left
                    } else {
                        SideToProcess::Right
                    };
                    up_edges.push(
                        org_eclipse_elk_core::org::eclipse::elk::core::util::pair::Pair::of(
                            side,
                            edge.clone(),
                        ),
                    );
                } else {
                    let side = if contains_port(left_ports, &target_port) {
                        SideToProcess::Left
                    } else {
                        SideToProcess::Right
                    };
                    down_edges.push(
                        org_eclipse_elk_core::org::eclipse::elk::core::util::pair::Pair::of(
                            side,
                            edge.clone(),
                        ),
                    );
                }
            }

            if up_edges.len() > 1 {
                let segment = crate::org::eclipse::elk::alg::layered::p5edges::splines::spline_segment::SplineSegment::new_hyper_edge(
                    single_port,
                    &up_edges,
                    side_to_process,
                );
                for pair in &up_edges {
                    self.edge_to_segment_map
                        .insert(edge_key(&pair.second), segment.clone());
                    remove_arc(&mut self.edges_remaining_layer, &pair.second);
                }
                self.spline_segments_layer.push(segment);
            }

            if down_edges.len() > 1 {
                let segment = crate::org::eclipse::elk::alg::layered::p5edges::splines::spline_segment::SplineSegment::new_hyper_edge(
                    single_port,
                    &down_edges,
                    side_to_process,
                );
                for pair in &down_edges {
                    self.edge_to_segment_map
                        .insert(edge_key(&pair.second), segment.clone());
                    remove_arc(&mut self.edges_remaining_layer, &pair.second);
                }
                self.spline_segments_layer.push(segment);
            }
        }
    }

    fn create_dependency(&self, edge0: &SplineSegmentRef, edge1: &SplineSegmentRef) {
        let (
            edge0_top,
            edge0_bottom,
            edge1_top,
            edge1_bottom,
            edge0_left_ports,
            edge0_right_ports,
            edge1_left_ports,
            edge1_right_ports,
        ) = {
            let edge0_guard = edge0.lock().ok();
            let edge1_guard = edge1.lock().ok();
            let Some(edge0_guard) = edge0_guard else {
                return;
            };
            let Some(edge1_guard) = edge1_guard else {
                return;
            };
            (
                edge0_guard.hyper_edge_top_y_pos,
                edge0_guard.hyper_edge_bottom_y_pos,
                edge1_guard.hyper_edge_top_y_pos,
                edge1_guard.hyper_edge_bottom_y_pos,
                edge0_guard.left_ports.clone(),
                edge0_guard.right_ports.clone(),
                edge1_guard.left_ports.clone(),
                edge1_guard.right_ports.clone(),
            )
        };

        if edge0_top > edge1_bottom || edge1_top > edge0_bottom {
            return;
        }

        let mut edge0_counter = 0i32;
        let mut edge1_counter = 0i32;

        for port in &edge0_right_ports {
            let port_y = port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.absolute_anchor())
                .map(|anchor| anchor.y)
                .unwrap_or(0.0);
            if SplinesMath::is_between(port_y, edge1_top, edge1_bottom) {
                edge0_counter += 1;
            }
        }
        for port in &edge0_left_ports {
            let port_y = port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.absolute_anchor())
                .map(|anchor| anchor.y)
                .unwrap_or(0.0);
            if SplinesMath::is_between(port_y, edge1_top, edge1_bottom) {
                edge0_counter -= 1;
            }
        }
        for port in &edge1_right_ports {
            let port_y = port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.absolute_anchor())
                .map(|anchor| anchor.y)
                .unwrap_or(0.0);
            if SplinesMath::is_between(port_y, edge0_top, edge0_bottom) {
                edge1_counter += 1;
            }
        }
        for port in &edge1_left_ports {
            let port_y = port
                .lock()
                .ok()
                .and_then(|port_guard| port_guard.absolute_anchor())
                .map(|anchor| anchor.y)
                .unwrap_or(0.0);
            if SplinesMath::is_between(port_y, edge0_top, edge0_bottom) {
                edge1_counter -= 1;
            }
        }

        if edge0_counter < edge1_counter {
            Dependency::new(edge0, edge1, edge1_counter - edge0_counter);
        } else if edge1_counter < edge0_counter {
            Dependency::new(edge1, edge0, edge0_counter - edge1_counter);
        } else {
            Dependency::new(edge1, edge0, 0);
            Dependency::new(edge0, edge1, 0);
        }
    }

    fn break_cycles(edges: &[SplineSegmentRef], random: &mut Random) {
        let mut sources: VecDeque<SplineSegmentRef> = VecDeque::new();
        let mut sinks: VecDeque<SplineSegmentRef> = VecDeque::new();

        let mut next_mark = -1;
        for edge in edges {
            if let Ok(mut edge_guard) = edge.lock() {
                edge_guard.mark = next_mark;
                next_mark -= 1;

                let mut inweight = 0;
                let mut outweight = 0;
                for dependency in edge_guard.outgoing.clone() {
                    outweight += dependency.lock().ok().map(|dep| dep.weight).unwrap_or(0);
                }
                for dependency in edge_guard.incoming.clone() {
                    inweight += dependency.lock().ok().map(|dep| dep.weight).unwrap_or(0);
                }

                edge_guard.inweight = inweight;
                edge_guard.outweight = outweight;

                if outweight == 0 {
                    sinks.push_back(edge.clone());
                } else if inweight == 0 {
                    sources.push_back(edge.clone());
                }
            }
        }

        let mut unprocessed: Vec<SplineSegmentRef> = edges.to_vec();
        let mark_base = edges.len() as i32;
        let mut next_left = mark_base + 1;
        let mut next_right = mark_base - 1;
        let mut max_edges: Vec<SplineSegmentRef> = Vec::new();

        while !unprocessed.is_empty() {
            while let Some(sink) = sources_pop(&mut sinks) {
                remove_segment(&mut unprocessed, &sink);
                if let Ok(mut guard) = sink.lock() {
                    guard.mark = next_right;
                    next_right -= 1;
                }
                Self::update_neighbors(&sink, &mut sources, &mut sinks);
            }

            while let Some(source) = sources_pop(&mut sources) {
                remove_segment(&mut unprocessed, &source);
                if let Ok(mut guard) = source.lock() {
                    guard.mark = next_left;
                    next_left += 1;
                }
                Self::update_neighbors(&source, &mut sources, &mut sinks);
            }

            let mut max_outflow = i32::MIN;
            for edge in &unprocessed {
                if let Ok(edge_guard) = edge.lock() {
                    let outflow = edge_guard.outweight - edge_guard.inweight;
                    if outflow >= max_outflow {
                        if outflow > max_outflow {
                            max_edges.clear();
                            max_outflow = outflow;
                        }
                        max_edges.push(edge.clone());
                    }
                }
            }

            if !max_edges.is_empty() {
                let index = random.next_int(max_edges.len() as i32) as usize;
                let max_edge = max_edges[index].clone();
                remove_segment(&mut unprocessed, &max_edge);
                if let Ok(mut guard) = max_edge.lock() {
                    guard.mark = next_left;
                    next_left += 1;
                }
                Self::update_neighbors(&max_edge, &mut sources, &mut sinks);
                max_edges.clear();
            }
        }

        let shift_base = edges.len() as i32 + 1;
        for edge in edges {
            if let Ok(mut guard) = edge.lock() {
                if guard.mark < mark_base {
                    guard.mark += shift_base;
                }
            }
        }

        for source in edges {
            let outgoing = source
                .lock()
                .ok()
                .map(|seg| seg.outgoing.clone())
                .unwrap_or_default();
            for dependency in outgoing {
                let (dep_source, dep_target, weight) = {
                    let dep_guard = dependency.lock().ok();
                    let Some(dep_guard) = dep_guard else {
                        continue;
                    };
                    (
                        dep_guard.source.clone(),
                        dep_guard.target.clone(),
                        dep_guard.weight,
                    )
                };

                let source_mark = dep_source.lock().ok().map(|seg| seg.mark).unwrap_or(0);
                let target_mark = dep_target.lock().ok().map(|seg| seg.mark).unwrap_or(0);
                if source_mark > target_mark {
                    if let Ok(mut seg) = dep_source.lock() {
                        seg.outgoing.retain(|dep| !Arc::ptr_eq(dep, &dependency));
                    }
                    if let Ok(mut seg) = dep_target.lock() {
                        seg.incoming.retain(|dep| !Arc::ptr_eq(dep, &dependency));
                    }

                    if weight > 0 {
                        if let Ok(mut dep_guard) = dependency.lock() {
                            dep_guard.source = dep_target.clone();
                            dep_guard.target = dep_source.clone();
                        }
                        if let Ok(mut seg) = dep_target.lock() {
                            seg.outgoing.push(dependency.clone());
                        }
                        if let Ok(mut seg) = dep_source.lock() {
                            seg.incoming.push(dependency.clone());
                        }
                    }
                }
            }
        }
    }

    fn update_neighbors(
        edge: &SplineSegmentRef,
        sources: &mut VecDeque<SplineSegmentRef>,
        sinks: &mut VecDeque<SplineSegmentRef>,
    ) {
        let outgoing = edge
            .lock()
            .ok()
            .map(|seg| seg.outgoing.clone())
            .unwrap_or_default();
        for dep in outgoing {
            let (target, weight, target_mark) = {
                let dep_guard = dep.lock().ok();
                let Some(dep_guard) = dep_guard else {
                    continue;
                };
                let target = dep_guard.target.clone();
                let weight = dep_guard.weight;
                let target_mark = target.lock().ok().map(|seg| seg.mark).unwrap_or(0);
                (target, weight, target_mark)
            };
            if target_mark < 0 && weight > 0 {
                if let Ok(mut guard) = target.lock() {
                    guard.inweight -= weight;
                    if guard.inweight <= 0 && guard.outweight > 0 {
                        sources.push_back(target.clone());
                    }
                }
            }
        }

        let incoming = edge
            .lock()
            .ok()
            .map(|seg| seg.incoming.clone())
            .unwrap_or_default();
        for dep in incoming {
            let (source, weight, source_mark) = {
                let dep_guard = dep.lock().ok();
                let Some(dep_guard) = dep_guard else {
                    continue;
                };
                let source = dep_guard.source.clone();
                let weight = dep_guard.weight;
                let source_mark = source.lock().ok().map(|seg| seg.mark).unwrap_or(0);
                (source, weight, source_mark)
            };
            if source_mark < 0 && weight > 0 {
                if let Ok(mut guard) = source.lock() {
                    guard.outweight -= weight;
                    if guard.outweight <= 0 && guard.inweight > 0 {
                        sinks.push_back(source.clone());
                    }
                }
            }
        }
    }

    fn topological_numbering(edges: &[SplineSegmentRef]) {
        let mut sources: VecDeque<SplineSegmentRef> = VecDeque::new();
        let mut rightward_targets: VecDeque<SplineSegmentRef> = VecDeque::new();
        for edge in edges {
            if let Ok(mut guard) = edge.lock() {
                guard.rank = 0;
                guard.inweight = guard.incoming.len() as i32;
                guard.outweight = guard.outgoing.len() as i32;

                if guard.inweight == 0 {
                    sources.push_back(edge.clone());
                }
                if guard.outweight == 0 && guard.left_ports.is_empty() {
                    rightward_targets.push_back(edge.clone());
                }
            }
        }

        let mut max_rank = -1;
        while let Some(edge) = sources.pop_front() {
            let outgoing = edge
                .lock()
                .ok()
                .map(|seg| seg.outgoing.clone())
                .unwrap_or_default();
            for dep in outgoing {
                let target = dep.lock().ok().map(|dep_guard| dep_guard.target.clone());
                let Some(target) = target else {
                    continue;
                };
                let mut target_guard = target.lock().ok();
                let Some(ref mut target_guard) = target_guard else {
                    continue;
                };
                let edge_rank = edge.lock().ok().map(|seg| seg.rank).unwrap_or(0);
                target_guard.rank = target_guard.rank.max(edge_rank + 1);
                max_rank = max_rank.max(target_guard.rank);
                target_guard.inweight -= 1;
                if target_guard.inweight == 0 {
                    sources.push_back(target.clone());
                }
            }
        }

        if max_rank > -1 {
            for edge in rightward_targets.iter() {
                if let Ok(mut guard) = edge.lock() {
                    guard.rank = max_rank;
                }
            }

            while let Some(edge) = rightward_targets.pop_front() {
                let incoming = edge
                    .lock()
                    .ok()
                    .map(|seg| seg.incoming.clone())
                    .unwrap_or_default();
                for dep in incoming {
                    let source = dep.lock().ok().map(|dep_guard| dep_guard.source.clone());
                    let Some(source) = source else {
                        continue;
                    };
                    let mut source_guard = source.lock().ok();
                    let Some(ref mut source_guard) = source_guard else {
                        continue;
                    };
                    if !source_guard.left_ports.is_empty() {
                        continue;
                    }
                    let edge_rank = edge.lock().ok().map(|seg| seg.rank).unwrap_or(0);
                    source_guard.rank = source_guard.rank.min(edge_rank - 1);
                    source_guard.outweight -= 1;
                    if source_guard.outweight == 0 {
                        rightward_targets.push_back(source.clone());
                    }
                }
            }
        }
    }

    fn get_edge_chain(&self, start: &LEdgeRef) -> Vec<LEdgeRef> {
        let mut edge_chain: Vec<LEdgeRef> = Vec::new();
        let mut current = start.clone();
        loop {
            edge_chain.push(current.clone());
            let next = self.successing_edge.get(&edge_key(&current)).cloned();
            match next {
                Some(next_edge) => current = next_edge,
                None => break,
            }
        }
        edge_chain
    }

    fn get_spline_path(&self, start: &LEdgeRef) -> Vec<SplineSegmentRef> {
        let mut segment_chain: Vec<SplineSegmentRef> = Vec::new();
        let mut current = start.clone();
        loop {
            let segment = self.edge_to_segment_map.get(&edge_key(&current)).cloned();
            let Some(segment) = segment else {
                break;
            };
            let (source_port, target_port) = {
                let edge_guard = current.lock().ok();
                let edge_guard = edge_guard.as_ref();
                (
                    edge_guard.and_then(|edge| edge.source()),
                    edge_guard.and_then(|edge| edge.target()),
                )
            };
            if let Ok(mut segment_guard) = segment.lock() {
                segment_guard.source_port = source_port.clone();
                segment_guard.target_port = target_port.clone();
            }
            segment_chain.push(segment);
            let next = self.successing_edge.get(&edge_key(&current)).cloned();
            match next {
                Some(next_edge) => current = next_edge,
                None => break,
            }
        }

        if let Some(first) = segment_chain.first() {
            if let Ok(mut guard) = first.lock() {
                guard.initial_segment = true;
                if let Some(edge) = guard.edges.first() {
                    guard.source_node = edge
                        .lock()
                        .ok()
                        .and_then(|edge_guard| edge_guard.source())
                        .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
                }
            }
        }
        if let Some(last) = segment_chain.last() {
            if let Ok(mut guard) = last.lock() {
                guard.last_segment = true;
                if let Some(edge) = guard.edges.first() {
                    guard.target_node = edge
                        .lock()
                        .ok()
                        .and_then(|edge_guard| edge_guard.target())
                        .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
                }
            }
        }
        segment_chain
    }

    pub fn is_straight(first_y: f64, second_y: f64) -> bool {
        (first_y - second_y).abs() < Self::MAX_VERTICAL_DIFF_FOR_STRAIGHT
    }

    pub fn is_normal_node(node: &LNode) -> bool {
        matches!(
            node.node_type(),
            crate::org::eclipse::elk::alg::layered::graph::NodeType::Normal
                | crate::org::eclipse::elk::alg::layered::graph::NodeType::BreakingPoint
        )
    }

    pub fn is_qualified_as_starting_node(node: &LNode) -> bool {
        matches!(
            node.node_type(),
            crate::org::eclipse::elk::alg::layered::graph::NodeType::Normal
                | crate::org::eclipse::elk::alg::layered::graph::NodeType::NorthSouthPort
                | crate::org::eclipse::elk::alg::layered::graph::NodeType::ExternalPort
                | crate::org::eclipse::elk::alg::layered::graph::NodeType::BreakingPoint
        )
    }
}

impl Default for SplineEdgeRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for SplineEdgeRouter {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Spline edge routing", 1.0);

        if layered_graph.layers().is_empty() {
            layered_graph.size().x = 0.0;
            monitor.done();
            return;
        }

        let node_node_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS)
            .unwrap_or(0.0);
        let edge_node_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS)
            .unwrap_or(0.0);
        let edge_edge_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_EDGE_BETWEEN_LAYERS)
            .unwrap_or(0.0);
        let spline_mode = layered_graph
            .get_property(LayeredOptions::EDGE_ROUTING_SPLINES_MODE)
            .unwrap_or(SplineRoutingMode::Sloppy);
        let sloppy_routing = spline_mode == SplineRoutingMode::Sloppy;
        let sloppy_layer_spacing_factor = layered_graph
            .get_property(LayeredOptions::EDGE_ROUTING_SPLINES_SLOPPY_LAYER_SPACING_FACTOR)
            .unwrap_or(0.0);

        self.start_edges.clear();
        self.all_spline_segments.clear();
        self.successing_edge.clear();
        self.edge_to_segment_map.clear();

        let mut random = layered_graph
            .get_property(InternalProperties::RANDOM)
            .unwrap_or_else(|| Random::new(0));

        let layers = layered_graph.layers().clone();
        let first_layer = layers.first().cloned();
        let last_layer = layers.last().cloned();
        let _is_left_layer_external = first_layer
            .as_ref()
            .map(|layer| {
                layer
                    .lock()
                    .ok()
                    .map(|layer_guard| {
                        layer_guard
                            .nodes()
                            .iter()
                            .all(PolylineEdgeRouter::is_external_west_or_east_port)
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        let is_right_layer_external = last_layer
            .as_ref()
            .map(|layer| {
                layer
                    .lock()
                    .ok()
                    .map(|layer_guard| {
                        layer_guard
                            .nodes()
                            .iter()
                            .all(PolylineEdgeRouter::is_external_west_or_east_port)
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false);

        let mut left_layer: Option<LayerRef> = None;
        let mut xpos = 0.0;
        let mut is_special_left_layer = true;

        for idx in 0..=layers.len() {
            let right_layer = layers.get(idx).cloned();
            self.clear_then_fill_mappings(left_layer.as_ref(), right_layer.as_ref());

            self.create_segments_and_compute_ranking(&mut random);

            let slot_count = self
                .spline_segments_layer
                .iter()
                .filter_map(|segment| {
                    segment.lock().ok().map(|seg| {
                        if seg.is_straight {
                            None
                        } else {
                            Some(seg.rank + 1)
                        }
                    })
                })
                .flatten()
                .max()
                .unwrap_or(0);

            let mut x_segment_delta = 0.0;
            let mut right_layer_position = xpos;
            let is_special_right_layer = right_layer.is_none()
                || (is_right_layer_external
                    && right_layer
                        .as_ref()
                        .map(|layer| {
                            last_layer
                                .as_ref()
                                .map(|last| Arc::ptr_eq(layer, last))
                                .unwrap_or(false)
                        })
                        .unwrap_or(false));

            if slot_count > 0 {
                let mut increment = 0.0;
                if left_layer.is_some() {
                    increment += edge_node_spacing;
                }
                increment += (slot_count.saturating_sub(1) as f64) * edge_edge_spacing;
                if right_layer.is_some() {
                    increment += edge_node_spacing;
                }
                if sloppy_routing {
                    if let Some(right_layer) = &right_layer {
                        increment = increment.max(self.compute_sloppy_spacing(
                            right_layer,
                            edge_edge_spacing,
                            node_node_spacing,
                            sloppy_layer_spacing_factor,
                        ));
                    }
                }
                if increment < node_node_spacing
                    && !is_special_left_layer
                    && !is_special_right_layer
                {
                    x_segment_delta = (node_node_spacing - increment) / 2.0;
                    increment = node_node_spacing;
                }
                right_layer_position += increment;
            } else if !is_special_left_layer && !is_special_right_layer {
                right_layer_position += node_node_spacing;
            }

            if let Some(right_layer) = &right_layer {
                LGraphUtil::place_nodes_horizontally(right_layer, right_layer_position);
            }

            for segment in &self.spline_segments_layer {
                if let Ok(mut seg) = segment.lock() {
                    seg.bounding_box.x = xpos;
                    seg.bounding_box.width = right_layer_position - xpos;
                    seg.x_delta = x_segment_delta;
                    seg.is_west_of_initial_layer = left_layer.is_none();
                }
            }
            self.all_spline_segments
                .extend(self.spline_segments_layer.iter().cloned());

            xpos = right_layer_position;
            if let Some(right_layer) = &right_layer {
                let layer_size_x = right_layer
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.size_ref().x)
                    .unwrap_or(0.0);
                xpos += layer_size_x;
            }

            left_layer = right_layer;
            is_special_left_layer = is_special_right_layer;
        }

        for edge in &self.start_edges {
            let edge_chain = self.get_edge_chain(edge);
            if let Ok(mut edge_guard) = edge.lock() {
                edge_guard.set_property(InternalProperties::SPLINE_EDGE_CHAIN, Some(edge_chain));
            }

            let spline = self.get_spline_path(edge);
            if let Ok(mut edge_guard) = edge.lock() {
                edge_guard.set_property(InternalProperties::SPLINE_ROUTE_START, Some(spline));
            }
        }

        layered_graph.size().x = xpos;
        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        let mut configuration = LayoutProcessorConfiguration::create();
        configuration.add_all(&BASELINE_PROCESSING_ADDITIONS);

        let graph_properties = graph
            .get_property_ref(InternalProperties::GRAPH_PROPERTIES)
            .unwrap_or_else(EnumSet::none_of);

        if graph_properties.contains(&GraphProperties::SelfLoops) {
            configuration.add_all(&SELF_LOOP_PROCESSING_ADDITIONS);
        }

        if graph_properties.contains(&GraphProperties::CenterLabels) {
            configuration.add_all(&CENTER_EDGE_LABEL_PROCESSING_ADDITIONS);
        }

        if graph_properties.contains(&GraphProperties::NorthSouthPorts) {
            configuration.add_all(&NORTH_SOUTH_PORT_PROCESSING_ADDITIONS);
        }

        if graph_properties.contains(&GraphProperties::EndLabels) {
            configuration.add_all(&END_EDGE_LABEL_PROCESSING_ADDITIONS);
        }

        Some(configuration)
    }
}

fn edge_key(edge: &LEdgeRef) -> usize {
    Arc::as_ptr(edge) as usize
}

fn contains_port(ports: &[LPortRef], port: &LPortRef) -> bool {
    ports.iter().any(|candidate| Arc::ptr_eq(candidate, port))
}

fn contains_edge(edges: &[LEdgeRef], edge: &LEdgeRef) -> bool {
    edges.iter().any(|candidate| Arc::ptr_eq(candidate, edge))
}

fn remove_arc<T>(items: &mut Vec<Arc<Mutex<T>>>, target: &Arc<Mutex<T>>) -> bool {
    if let Some(pos) = items.iter().position(|item| Arc::ptr_eq(item, target)) {
        items.remove(pos);
        true
    } else {
        false
    }
}

fn remove_segment(list: &mut Vec<SplineSegmentRef>, target: &SplineSegmentRef) {
    if let Some(pos) = list.iter().position(|item| Arc::ptr_eq(item, target)) {
        list.remove(pos);
    }
}

fn sources_pop(queue: &mut VecDeque<SplineSegmentRef>) -> Option<SplineSegmentRef> {
    queue.pop_front()
}
