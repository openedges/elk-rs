use std::cell::RefCell;
use rustc_hash::FxHashMap;
use std::rc::{Rc, Weak};
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::pair::Pair;

use crate::org::eclipse::elk::alg::layered::graph::LPortRef;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::direction::routing_direction::RoutingDirection;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment_dependency::HyperEdgeSegmentDependency;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal::hyper_edge_segment_dependency::HyperEdgeSegmentDependencyRef;

pub type HyperEdgeSegmentRef = Rc<RefCell<HyperEdgeSegment>>;

pub struct HyperEdgeSegment {
    routing_direction: RoutingDirection,
    ports: Vec<LPortRef>,
    pub(crate) mark: i32,
    routing_slot: i32,
    start_position: f64,
    end_position: f64,
    incoming_connection_coordinates: Vec<f64>,
    outgoing_connection_coordinates: Vec<f64>,
    outgoing_segment_dependencies: Vec<HyperEdgeSegmentDependencyRef>,
    incoming_segment_dependencies: Vec<HyperEdgeSegmentDependencyRef>,
    out_dep_weight: i32,
    critical_out_dep_weight: i32,
    in_dep_weight: i32,
    critical_in_dep_weight: i32,
    split_partner: Option<Weak<RefCell<HyperEdgeSegment>>>,
    split_by: Option<Weak<RefCell<HyperEdgeSegment>>>,
}

impl HyperEdgeSegment {
    pub fn new(direction: RoutingDirection) -> HyperEdgeSegmentRef {
        Rc::new(RefCell::new(HyperEdgeSegment {
            routing_direction: direction,
            ports: Vec::new(),
            mark: 0,
            routing_slot: 0,
            start_position: f64::NAN,
            end_position: f64::NAN,
            incoming_connection_coordinates: Vec::new(),
            outgoing_connection_coordinates: Vec::new(),
            outgoing_segment_dependencies: Vec::new(),
            incoming_segment_dependencies: Vec::new(),
            out_dep_weight: 0,
            critical_out_dep_weight: 0,
            in_dep_weight: 0,
            critical_in_dep_weight: 0,
            split_partner: None,
            split_by: None,
        }))
    }

    pub fn add_port_positions(
        segment_ref: &HyperEdgeSegmentRef,
        port: &LPortRef,
        hyper_edge_segment_map: &mut FxHashMap<usize, HyperEdgeSegmentRef>,
    ) {
        hyper_edge_segment_map.insert(port_key(port), segment_ref.clone());
        let mut segment = segment_ref.borrow_mut();
        segment.ports.push(port.clone());

        let port_pos = segment.get_port_position_on_hyper_node(port);
        if port
            .lock_ok()
            .map(|port_guard| port_guard.side() == segment.source_port_side())
            .unwrap_or(false)
        {
            insert_sorted(&mut segment.incoming_connection_coordinates, port_pos);
        } else {
            insert_sorted(&mut segment.outgoing_connection_coordinates, port_pos);
        }
        segment.recompute_extent();
        drop(segment);

        let connected_ports = port
            .lock().connected_ports();
        for other_port in connected_ports {
            if !hyper_edge_segment_map.contains_key(&port_key(&other_port)) {
                HyperEdgeSegment::add_port_positions(
                    segment_ref,
                    &other_port,
                    hyper_edge_segment_map,
                );
            }
        }
    }

    fn get_port_position_on_hyper_node(&self, port: &LPortRef) -> f64 {
        let Some(mut port_guard) = port.lock_ok() else {
            return 0.0;
        };
        let node_pos = port_guard
            .node()
            .and_then(|node| {
                node.lock_ok()
                    .map(|mut node_guard| *node_guard.shape().position_ref())
            })
            .unwrap_or_default();
        let port_pos = *port_guard.shape().position_ref();
        let anchor = *port_guard.anchor_ref();

        match self.routing_direction {
            RoutingDirection::WestToEast => node_pos.y + port_pos.y + anchor.y,
            RoutingDirection::NorthToSouth | RoutingDirection::SouthToNorth => {
                node_pos.x + port_pos.x + anchor.x
            }
        }
    }

    fn source_port_side(&self) -> PortSide {
        match self.routing_direction {
            RoutingDirection::WestToEast => PortSide::East,
            RoutingDirection::NorthToSouth => PortSide::South,
            RoutingDirection::SouthToNorth => PortSide::North,
        }
    }

    pub fn routing_slot(&self) -> i32 {
        self.routing_slot
    }

    pub fn set_routing_slot(&mut self, slot: i32) {
        self.routing_slot = slot;
    }

    pub fn start_coordinate(&self) -> f64 {
        self.start_position
    }

    pub fn end_coordinate(&self) -> f64 {
        self.end_position
    }

    pub fn incoming_connection_coordinates(&self) -> &Vec<f64> {
        &self.incoming_connection_coordinates
    }

    pub fn outgoing_connection_coordinates(&self) -> &Vec<f64> {
        &self.outgoing_connection_coordinates
    }

    pub fn incoming_connection_coordinates_mut(&mut self) -> &mut Vec<f64> {
        &mut self.incoming_connection_coordinates
    }

    pub fn outgoing_connection_coordinates_mut(&mut self) -> &mut Vec<f64> {
        &mut self.outgoing_connection_coordinates
    }

    pub fn outgoing_segment_dependencies(&self) -> &Vec<HyperEdgeSegmentDependencyRef> {
        &self.outgoing_segment_dependencies
    }

    pub fn incoming_segment_dependencies(&self) -> &Vec<HyperEdgeSegmentDependencyRef> {
        &self.incoming_segment_dependencies
    }

    pub fn add_outgoing_dependency(&mut self, dependency: HyperEdgeSegmentDependencyRef) {
        self.outgoing_segment_dependencies.push(dependency);
    }

    pub fn add_incoming_dependency(&mut self, dependency: HyperEdgeSegmentDependencyRef) {
        self.incoming_segment_dependencies.push(dependency);
    }

    pub fn remove_outgoing_dependency(&mut self, dependency: &HyperEdgeSegmentDependencyRef) {
        if let Some(pos) = self.outgoing_segment_dependencies.iter().position(|dep| Rc::ptr_eq(dep, dependency)) {
            self.outgoing_segment_dependencies.swap_remove(pos);
        }
    }

    pub fn remove_incoming_dependency(&mut self, dependency: &HyperEdgeSegmentDependencyRef) {
        if let Some(pos) = self.incoming_segment_dependencies.iter().position(|dep| Rc::ptr_eq(dep, dependency)) {
            self.incoming_segment_dependencies.swap_remove(pos);
        }
    }

    pub fn out_weight(&self) -> i32 {
        self.out_dep_weight
    }

    pub fn set_out_weight(&mut self, weight: i32) {
        self.out_dep_weight = weight;
    }

    pub fn critical_out_weight(&self) -> i32 {
        self.critical_out_dep_weight
    }

    pub fn set_critical_out_weight(&mut self, weight: i32) {
        self.critical_out_dep_weight = weight;
    }

    pub fn in_weight(&self) -> i32 {
        self.in_dep_weight
    }

    pub fn set_in_weight(&mut self, weight: i32) {
        self.in_dep_weight = weight;
    }

    pub fn critical_in_weight(&self) -> i32 {
        self.critical_in_dep_weight
    }

    pub fn set_critical_in_weight(&mut self, weight: i32) {
        self.critical_in_dep_weight = weight;
    }

    pub fn split_partner(&self) -> Option<HyperEdgeSegmentRef> {
        self.split_partner
            .as_ref()
            .and_then(|partner| partner.upgrade())
    }

    pub fn set_split_partner(&mut self, split_partner: Option<&HyperEdgeSegmentRef>) {
        self.split_partner = split_partner.map(Rc::downgrade);
    }

    pub fn split_by(&self) -> Option<HyperEdgeSegmentRef> {
        self.split_by.as_ref().and_then(|partner| partner.upgrade())
    }

    pub fn set_split_by(&mut self, split_by: Option<&HyperEdgeSegmentRef>) {
        self.split_by = split_by.map(Rc::downgrade);
    }

    pub fn length(&self) -> f64 {
        self.end_position - self.start_position
    }

    pub fn represents_hyperedge(&self) -> bool {
        self.incoming_connection_coordinates.len() + self.outgoing_connection_coordinates.len() > 2
    }

    pub fn is_dummy(&self) -> bool {
        self.split_partner.is_some() && self.split_by.is_none()
    }

    pub fn recompute_extent(&mut self) {
        self.start_position = f64::NAN;
        self.end_position = f64::NAN;

        recompute_extent_list(
            &mut self.start_position,
            &mut self.end_position,
            &self.incoming_connection_coordinates,
        );
        recompute_extent_list(
            &mut self.start_position,
            &mut self.end_position,
            &self.outgoing_connection_coordinates,
        );
    }

    pub fn simulate_split(&self) -> Pair<HyperEdgeSegmentRef, HyperEdgeSegmentRef> {
        let new_split = HyperEdgeSegment::new(self.routing_direction);
        let new_split_partner = HyperEdgeSegment::new(self.routing_direction);

        {
            let mut split = new_split.borrow_mut();
            split.incoming_connection_coordinates = self.incoming_connection_coordinates.clone();
            split.split_by = self.split_by.clone();
            split.split_partner = Some(Rc::downgrade(&new_split_partner));
            split.recompute_extent();
        }

        {
            let mut partner = new_split_partner.borrow_mut();
            partner.outgoing_connection_coordinates = self.outgoing_connection_coordinates.clone();
            partner.split_partner = Some(Rc::downgrade(&new_split));
            partner.recompute_extent();
        }

        Pair::of(new_split, new_split_partner)
    }

    pub fn split_at(segment_ref: &HyperEdgeSegmentRef, split_position: f64) -> HyperEdgeSegmentRef {
        let direction = segment_ref.borrow().routing_direction;
        let split_partner = HyperEdgeSegment::new(direction);

        let (incoming_deps, outgoing_deps) = {
            let mut segment = segment_ref.borrow_mut();
            segment.split_partner = Some(Rc::downgrade(&split_partner));

            {
                let mut partner = split_partner.borrow_mut();
                partner.split_partner = Some(Rc::downgrade(segment_ref));
                partner.outgoing_connection_coordinates =
                    segment.outgoing_connection_coordinates.clone();
            }

            segment.outgoing_connection_coordinates.clear();
            segment.outgoing_connection_coordinates.push(split_position);
            split_partner
                .borrow_mut()
                .incoming_connection_coordinates
                .push(split_position);

            segment.recompute_extent();
            split_partner.borrow_mut().recompute_extent();

            (
                segment.incoming_segment_dependencies.clone(),
                segment.outgoing_segment_dependencies.clone(),
            )
        };

        for dep in incoming_deps {
            HyperEdgeSegmentDependency::remove(&dep);
        }
        for dep in outgoing_deps {
            HyperEdgeSegmentDependency::remove(&dep);
        }

        split_partner
    }

    pub fn ports(&self) -> &Vec<LPortRef> {
        &self.ports
    }
}

fn port_key(port: &LPortRef) -> usize {
    Arc::as_ptr(port) as usize
}

fn insert_sorted(list: &mut Vec<f64>, value: f64) {
    let mut insert_at = list.len();
    for (idx, existing) in list.iter().enumerate() {
        if (*existing - value).abs() < f64::EPSILON {
            return;
        }
        if *existing > value {
            insert_at = idx;
            break;
        }
    }
    list.insert(insert_at, value);
}

fn recompute_extent_list(start: &mut f64, end: &mut f64, positions: &[f64]) {
    if positions.is_empty() {
        return;
    }
    if start.is_nan() {
        *start = positions[0];
    } else {
        *start = start.min(positions[0]);
    }
    if end.is_nan() {
        *end = positions[positions.len() - 1];
    } else {
        *end = end.max(positions[positions.len() - 1]);
    }
}
