use rustc_hash::FxHashMap;
use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_rectangle::ElkRectangle;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::{
    PortSide, SIDES_NORTH_SOUTH,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::pair::Pair;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LNodeRef, LPortRef};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;
use crate::org::eclipse::elk::alg::layered::p5edges::splines::spline_edge_router::SplineEdgeRouter;

#[derive(Clone, Copy, Debug, Default)]
pub struct EdgeInformation {
    pub start_y: f64,
    pub end_y: f64,
    pub normal_source_node: bool,
    pub normal_target_node: bool,
    pub inverted_left: bool,
    pub inverted_right: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideToProcess {
    Left,
    Right,
}

pub type SplineSegmentRef = Arc<Mutex<SplineSegment>>;
pub type DependencyRef = Arc<Mutex<Dependency>>;

pub struct Dependency {
    pub source: SplineSegmentRef,
    pub target: SplineSegmentRef,
    pub weight: i32,
}

impl Dependency {
    pub fn new(source: &SplineSegmentRef, target: &SplineSegmentRef, weight: i32) -> DependencyRef {
        let dependency = Arc::new(Mutex::new(Dependency {
            source: source.clone(),
            target: target.clone(),
            weight,
        }));
        {
            let mut source_guard = source.lock();
            source_guard.outgoing.push(dependency.clone());
        }
        {
            let mut target_guard = target.lock();
            target_guard.incoming.push(dependency.clone());
        }
        dependency
    }
}

pub struct SplineSegment {
    pub handled: bool,
    pub left_ports: Vec<LPortRef>,
    pub right_ports: Vec<LPortRef>,
    pub outgoing: Vec<DependencyRef>,
    pub incoming: Vec<DependencyRef>,
    pub mark: i32,
    pub inweight: i32,
    pub outweight: i32,
    pub rank: i32,
    pub edges: Vec<LEdgeRef>,
    pub is_straight: bool,
    pub bounding_box: ElkRectangle,
    pub is_west_of_initial_layer: bool,
    pub x_delta: f64,
    pub source_port: Option<LPortRef>,
    pub target_port: Option<LPortRef>,
    pub initial_segment: bool,
    pub last_segment: bool,
    pub source_node: Option<LNodeRef>,
    pub target_node: Option<LNodeRef>,
    pub inverse_order: bool,
    pub hyper_edge_top_y_pos: f64,
    pub hyper_edge_bottom_y_pos: f64,
    pub center_control_point_y: f64,
    pub edge_information: FxHashMap<usize, EdgeInformation>,
}

impl SplineSegment {
    pub fn new_hyper_edge(
        single_port: &LPortRef,
        edges: &[Pair<SideToProcess, LEdgeRef>],
        source_side: SideToProcess,
    ) -> SplineSegmentRef {
        let mut segment = SplineSegment::empty();
        match source_side {
            SideToProcess::Left => Self::insert_unique_port(&mut segment.left_ports, single_port),
            SideToProcess::Right => Self::insert_unique_port(&mut segment.right_ports, single_port),
        }

        let mut y_min_target = f64::INFINITY;
        let mut y_max_target = f64::NEG_INFINITY;
        for pair in edges {
            let side = pair.first;
            let edge = pair.second.clone();
            let target_port = {
                let edge_guard = edge.lock();
                let source_port = edge_guard.source();
                let target_port = edge_guard.target();
                if let (Some(source_port), Some(target_port)) = (source_port, target_port) {
                    if Arc::ptr_eq(&source_port, single_port) {
                        target_port
                    } else {
                        source_port
                    }
                } else {
                    continue;
                }
            };

            match side {
                SideToProcess::Left => {
                    Self::insert_unique_port(&mut segment.left_ports, &target_port)
                }
                SideToProcess::Right => {
                    Self::insert_unique_port(&mut segment.right_ports, &target_port)
                }
            }

            let y_pos_target = Self::anchor_y(&target_port);
            y_min_target = y_min_target.min(y_pos_target);
            y_max_target = y_max_target.max(y_pos_target);
        }

        let y_pos_single = Self::anchor_y(single_port);
        segment.set_relevant_positions(y_pos_single, y_min_target, y_max_target);

        for pair in edges {
            segment.add_edge(&pair.second);
        }
        segment.is_straight = false;

        Arc::new(Mutex::new(segment))
    }

    pub fn new_single_edge(
        edge: &LEdgeRef,
        source_side: SideToProcess,
        target_side: SideToProcess,
    ) -> SplineSegmentRef {
        let mut segment = SplineSegment::empty();
        let (source_port, target_port) = {
            let edge_guard = edge.lock();
            (
                edge_guard.source(),
                edge_guard.target(),
            )
        };
        if let (Some(source_port), Some(target_port)) = (source_port, target_port) {
            match source_side {
                SideToProcess::Left => {
                    Self::insert_unique_port(&mut segment.left_ports, &source_port)
                }
                SideToProcess::Right => {
                    Self::insert_unique_port(&mut segment.right_ports, &source_port)
                }
            }
            match target_side {
                SideToProcess::Left => {
                    Self::insert_unique_port(&mut segment.left_ports, &target_port)
                }
                SideToProcess::Right => {
                    Self::insert_unique_port(&mut segment.right_ports, &target_port)
                }
            }

            segment.add_edge(edge);

            let source_y = Self::anchor_y(&source_port);
            let target_y = Self::anchor_y(&target_port);
            segment.set_relevant_positions(source_y, target_y, target_y);
            segment.is_straight = SplineEdgeRouter::is_straight(source_y, target_y);
        }

        Arc::new(Mutex::new(segment))
    }

    pub fn is_hyper_edge(&self) -> bool {
        self.edges.len() > 1
    }

    fn empty() -> SplineSegment {
        SplineSegment {
            handled: false,
            left_ports: Vec::new(),
            right_ports: Vec::new(),
            outgoing: Vec::new(),
            incoming: Vec::new(),
            mark: 0,
            inweight: 0,
            outweight: 0,
            rank: 0,
            edges: Vec::new(),
            is_straight: false,
            bounding_box: ElkRectangle::new(),
            is_west_of_initial_layer: false,
            x_delta: 0.0,
            source_port: None,
            target_port: None,
            initial_segment: false,
            last_segment: false,
            source_node: None,
            target_node: None,
            inverse_order: false,
            hyper_edge_top_y_pos: 0.0,
            hyper_edge_bottom_y_pos: 0.0,
            center_control_point_y: 0.0,
            edge_information: FxHashMap::default(),
        }
    }

    fn add_edge(&mut self, edge: &LEdgeRef) {
        if self
            .edges
            .iter()
            .any(|candidate| Arc::ptr_eq(candidate, edge))
        {
            return;
        }
        self.edges.push(edge.clone());
        let mut info = EdgeInformation::default();

        let (source_port, target_port) = {
            let edge_guard = edge.lock();
            (
                edge_guard.source(),
                edge_guard.target(),
            )
        };

        if let (Some(source_port), Some(target_port)) = (source_port, target_port) {
            info.start_y = Self::anchor_y(&source_port);
            info.end_y = Self::anchor_y(&target_port);

            let source_node = source_port
                .lock().node();
            let target_node = target_port
                .lock().node();
            info.normal_source_node = source_node
                .as_ref()
                .map(|node| SplineEdgeRouter::is_normal_node(&node.lock()))
                .unwrap_or(false);
            info.normal_target_node = target_node
                .as_ref()
                .map(|node| SplineEdgeRouter::is_normal_node(&node.lock()))
                .unwrap_or(false);

            info.inverted_left = source_port.lock().side() == PortSide::West;
            info.inverted_right = target_port.lock().side() == PortSide::East;
        }

        self.edge_information.insert(edge_key(edge), info);
    }

    fn anchor_y(port: &LPortRef) -> f64 {
        let mut port_guard = port.lock();
        let side = port_guard.side();
        if SIDES_NORTH_SOUTH.contains(&side) {
            port_guard
                .get_property(InternalProperties::SPLINE_NS_PORT_Y_COORD)
                .unwrap_or_else(|| {
                    port_guard
                        .absolute_anchor()
                        .map(|anchor| anchor.y)
                        .unwrap_or(0.0)
                })
        } else {
            port_guard
                .absolute_anchor()
                .map(|anchor| anchor.y)
                .unwrap_or(0.0)
        }
    }

    fn set_relevant_positions(&mut self, source_y: f64, target_y_min: f64, target_y_max: f64) {
        const HYPEREDGE_POS_OUTER_RATE: f64 = 0.9;
        const HYPEREDGE_POS_MID_RATE: f64 = 1.0 - HYPEREDGE_POS_OUTER_RATE;
        const ONE_HALF: f64 = 0.5;

        self.bounding_box.y = source_y.min(target_y_min);
        self.bounding_box.height = source_y.max(target_y_max) - self.bounding_box.y;

        if source_y < target_y_min {
            self.center_control_point_y = ONE_HALF * (source_y + target_y_min);
            self.hyper_edge_top_y_pos = HYPEREDGE_POS_MID_RATE * self.center_control_point_y
                + HYPEREDGE_POS_OUTER_RATE * source_y;
            self.hyper_edge_bottom_y_pos = HYPEREDGE_POS_MID_RATE * self.center_control_point_y
                + HYPEREDGE_POS_OUTER_RATE * target_y_min;
        } else {
            self.center_control_point_y = ONE_HALF * (source_y + target_y_max);
            self.hyper_edge_top_y_pos = HYPEREDGE_POS_MID_RATE * self.center_control_point_y
                + HYPEREDGE_POS_OUTER_RATE * target_y_max;
            self.hyper_edge_bottom_y_pos = HYPEREDGE_POS_MID_RATE * self.center_control_point_y
                + HYPEREDGE_POS_OUTER_RATE * source_y;
        }
    }

    fn insert_unique_port(ports: &mut Vec<LPortRef>, port: &LPortRef) {
        if ports.iter().any(|candidate| Arc::ptr_eq(candidate, port)) {
            return;
        }
        ports.push(port.clone());
    }
}

fn edge_key(edge: &LEdgeRef) -> usize {
    Arc::as_ptr(edge) as usize
}
