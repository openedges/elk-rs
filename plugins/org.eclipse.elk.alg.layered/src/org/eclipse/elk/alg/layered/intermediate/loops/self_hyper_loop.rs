use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSetType;

use crate::org::eclipse::elk::alg::layered::intermediate::loops::{
    SelfHyperLoopLabels, SelfLoopEdgeRef, SelfLoopPortRef,
};

pub type SelfHyperLoopRef = Arc<Mutex<SelfHyperLoop>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelfLoopType {
    OneSide,
    TwoSidesCorner,
    TwoSidesOpposing,
    ThreeSides,
    FourSides,
}

impl SelfLoopType {
    pub fn from_port_sides(port_sides: &HashSet<PortSide>) -> Option<SelfLoopType> {
        if port_sides.contains(&PortSide::Undefined) {
            return None;
        }

        match port_sides.len() {
            1 => Some(SelfLoopType::OneSide),
            2 => {
                let east_west =
                    port_sides.contains(&PortSide::East) && port_sides.contains(&PortSide::West);
                let north_south =
                    port_sides.contains(&PortSide::North) && port_sides.contains(&PortSide::South);
                if east_west || north_south {
                    Some(SelfLoopType::TwoSidesOpposing)
                } else {
                    Some(SelfLoopType::TwoSidesCorner)
                }
            }
            3 => Some(SelfLoopType::ThreeSides),
            4 => Some(SelfLoopType::FourSides),
            _ => None,
        }
    }
}

pub struct SelfHyperLoop {
    sl_ports: Vec<SelfLoopPortRef>,
    sl_edges: Vec<SelfLoopEdgeRef>,
    sl_labels: Option<SelfHyperLoopLabels>,
    self_loop_type: Option<SelfLoopType>,
    sl_ports_by_side: HashMap<PortSide, Vec<SelfLoopPortRef>>,
    sl_ports_by_side_order: Vec<PortSide>,
    leftmost_port: Option<SelfLoopPortRef>,
    rightmost_port: Option<SelfLoopPortRef>,
    occupied_port_sides: HashSet<PortSide>,
    routing_slot: Vec<i32>,
}

impl SelfHyperLoop {
    pub fn new() -> SelfHyperLoopRef {
        Arc::new(Mutex::new(SelfHyperLoop {
            sl_ports: Vec::new(),
            sl_edges: Vec::new(),
            sl_labels: None,
            self_loop_type: None,
            sl_ports_by_side: HashMap::new(),
            sl_ports_by_side_order: Vec::new(),
            leftmost_port: None,
            rightmost_port: None,
            occupied_port_sides: HashSet::new(),
            routing_slot: vec![0; PortSide::variants().len()],
        }))
    }

    pub fn add_self_loop_edge(sl_loop: &SelfHyperLoopRef, sl_edge: &SelfLoopEdgeRef) {
        {
            if let Ok(loop_guard) = sl_loop.lock() {
                if loop_guard
                    .sl_edges
                    .iter()
                    .any(|existing| Arc::ptr_eq(existing, sl_edge))
                {
                    return;
                }
            }
        }

        if let Ok(mut edge_guard) = sl_edge.lock() {
            edge_guard.set_sl_hyper_loop(sl_loop);
        }

        if let Ok(mut loop_guard) = sl_loop.lock() {
            loop_guard.sl_edges.push(sl_edge.clone());

            let (sl_source, sl_target, edge_labels) = sl_edge
                .lock()
                .ok()
                .map(|edge_guard| {
                    (
                        edge_guard.sl_source().clone(),
                        edge_guard.sl_target().clone(),
                        edge_guard
                            .l_edge()
                            .lock()
                            .ok()
                            .map(|edge| edge.labels().clone())
                            .unwrap_or_default(),
                    )
                })
                .unwrap_or_else(|| panic!("self loop edge lock poisoned"));

            if !loop_guard
                .sl_ports
                .iter()
                .any(|existing| Arc::ptr_eq(existing, &sl_source))
            {
                loop_guard.sl_ports.push(sl_source);
            }
            if !loop_guard
                .sl_ports
                .iter()
                .any(|existing| Arc::ptr_eq(existing, &sl_target))
            {
                loop_guard.sl_ports.push(sl_target);
            }

            if !edge_labels.is_empty() {
                if loop_guard.sl_labels.is_none() {
                    loop_guard.sl_labels = Some(SelfHyperLoopLabels::new());
                }
                if let Some(sl_labels) = loop_guard.sl_labels.as_mut() {
                    sl_labels.add_l_labels(&edge_labels);
                }
            }
        }
    }

    pub fn sl_ports(&self) -> &Vec<SelfLoopPortRef> {
        &self.sl_ports
    }

    pub fn sl_ports_mut(&mut self) -> &mut Vec<SelfLoopPortRef> {
        &mut self.sl_ports
    }

    pub fn sl_edges(&self) -> &Vec<SelfLoopEdgeRef> {
        &self.sl_edges
    }

    pub fn sl_labels(&self) -> Option<&SelfHyperLoopLabels> {
        self.sl_labels.as_ref()
    }

    pub fn sl_labels_mut(&mut self) -> Option<&mut SelfHyperLoopLabels> {
        self.sl_labels.as_mut()
    }

    pub fn compute_ports_per_side(&mut self) {
        self.sl_ports_by_side.clear();
        self.sl_ports_by_side_order.clear();
        let mut sides = HashSet::new();

        for sl_port in &self.sl_ports {
            let side = sl_port_side(sl_port);
            if side == PortSide::Undefined {
                continue;
            }
            if !self.sl_ports_by_side_order.contains(&side) {
                self.sl_ports_by_side_order.push(side);
            }
            self.sl_ports_by_side
                .entry(side)
                .or_default()
                .push(sl_port.clone());
            sides.insert(side);
        }

        self.self_loop_type = SelfLoopType::from_port_sides(&sides);
    }

    pub fn self_loop_type(&self) -> Option<SelfLoopType> {
        self.self_loop_type
    }

    pub fn ports_on_side(&self, side: PortSide) -> Vec<SelfLoopPortRef> {
        self.sl_ports_by_side
            .get(&side)
            .cloned()
            .unwrap_or_default()
    }

    pub fn port_sides_in_insertion_order(&self) -> Vec<PortSide> {
        self.sl_ports_by_side_order.clone()
    }

    pub fn leftmost_port(&self) -> Option<SelfLoopPortRef> {
        self.leftmost_port.clone()
    }

    pub fn set_leftmost_port(&mut self, sl_port: Option<SelfLoopPortRef>) {
        self.leftmost_port = sl_port;
    }

    pub fn rightmost_port(&self) -> Option<SelfLoopPortRef> {
        self.rightmost_port.clone()
    }

    pub fn set_rightmost_port(&mut self, sl_port: Option<SelfLoopPortRef>) {
        self.rightmost_port = sl_port;
    }

    pub fn occupied_port_sides(&self) -> &HashSet<PortSide> {
        &self.occupied_port_sides
    }

    pub fn set_occupied_port_sides(&mut self, sides: HashSet<PortSide>) {
        self.occupied_port_sides = sides;
    }

    pub fn clear_routing_slots(&mut self) {
        for slot in &mut self.routing_slot {
            *slot = 0;
        }
    }

    pub fn routing_slot(&self, port_side: PortSide) -> i32 {
        self.routing_slot[side_index(port_side)]
    }

    pub fn set_routing_slot(&mut self, port_side: PortSide, slot: i32) {
        self.routing_slot[side_index(port_side)] = slot;
    }

    pub fn sort_ports_by_id(&mut self) {
        self.sl_ports.sort_by_key(Self::port_id);
    }

    pub fn port_id(sl_port: &SelfLoopPortRef) -> i32 {
        sl_port
            .lock()
            .ok()
            .and_then(|port_guard| {
                port_guard
                    .l_port()
                    .lock()
                    .ok()
                    .map(|mut l_port_guard| l_port_guard.shape().graph_element().id)
            })
            .unwrap_or(i32::MAX)
    }
}

fn sl_port_side(sl_port: &SelfLoopPortRef) -> PortSide {
    sl_port
        .lock()
        .ok()
        .and_then(|port_guard| {
            port_guard
                .l_port()
                .lock()
                .ok()
                .map(|l_port_guard| l_port_guard.side())
        })
        .unwrap_or(PortSide::Undefined)
}

fn side_index(side: PortSide) -> usize {
    match side {
        PortSide::Undefined => 0,
        PortSide::North => 1,
        PortSide::East => 2,
        PortSide::South => 3,
        PortSide::West => 4,
    }
}
