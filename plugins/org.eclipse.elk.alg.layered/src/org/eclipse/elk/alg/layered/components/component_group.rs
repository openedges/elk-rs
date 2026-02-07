use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

use crate::org::eclipse::elk::alg::layered::graph::LGraphRef;
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;

const MASK_NONE: u8 = 0;
const MASK_NORTH: u8 = 0b0001;
const MASK_EAST: u8 = 0b0010;
const MASK_SOUTH: u8 = 0b0100;
const MASK_WEST: u8 = 0b1000;

const MASK_NORTH_SOUTH: u8 = MASK_NORTH | MASK_SOUTH;
const MASK_EAST_WEST: u8 = MASK_EAST | MASK_WEST;
const MASK_NORTH_WEST: u8 = MASK_NORTH | MASK_WEST;
const MASK_NORTH_EAST: u8 = MASK_NORTH | MASK_EAST;
const MASK_SOUTH_WEST: u8 = MASK_SOUTH | MASK_WEST;
const MASK_EAST_SOUTH: u8 = MASK_EAST | MASK_SOUTH;
const MASK_NORTH_EAST_WEST: u8 = MASK_NORTH | MASK_EAST | MASK_WEST;
const MASK_EAST_SOUTH_WEST: u8 = MASK_EAST | MASK_SOUTH | MASK_WEST;
const MASK_NORTH_SOUTH_WEST: u8 = MASK_NORTH | MASK_SOUTH | MASK_WEST;
const MASK_NORTH_EAST_SOUTH: u8 = MASK_NORTH | MASK_EAST | MASK_SOUTH;
const MASK_NORTH_EAST_SOUTH_WEST: u8 = MASK_NORTH | MASK_EAST | MASK_SOUTH | MASK_WEST;

const CONSTRAINT_NONE: [u8; 1] = [MASK_NORTH_EAST_SOUTH_WEST];
const CONSTRAINT_WEST: [u8; 2] = [MASK_NORTH_EAST_SOUTH_WEST, MASK_NORTH_SOUTH_WEST];
const CONSTRAINT_EAST: [u8; 2] = [MASK_NORTH_EAST_SOUTH, MASK_NORTH_EAST_SOUTH_WEST];
const CONSTRAINT_NORTH: [u8; 2] = [MASK_NORTH_EAST_SOUTH_WEST, MASK_NORTH_EAST_WEST];
const CONSTRAINT_SOUTH: [u8; 2] = [MASK_EAST_SOUTH_WEST, MASK_NORTH_EAST_SOUTH_WEST];
const CONSTRAINT_NORTH_SOUTH: [u8; 4] = [
    MASK_EAST_WEST,
    MASK_NORTH_EAST_SOUTH_WEST,
    MASK_NORTH_EAST_WEST,
    MASK_EAST_SOUTH_WEST,
];
const CONSTRAINT_EAST_WEST: [u8; 4] = [
    MASK_NORTH_SOUTH,
    MASK_NORTH_SOUTH_WEST,
    MASK_NORTH_EAST_SOUTH,
    MASK_NORTH_EAST_SOUTH_WEST,
];
const CONSTRAINT_NORTH_WEST: [u8; 3] = [
    MASK_NORTH_WEST,
    MASK_NORTH_EAST_WEST,
    MASK_NORTH_SOUTH_WEST,
];
const CONSTRAINT_NORTH_EAST: [u8; 3] = [
    MASK_NORTH_EAST,
    MASK_NORTH_EAST_WEST,
    MASK_NORTH_EAST_SOUTH,
];
const CONSTRAINT_SOUTH_WEST: [u8; 3] = [
    MASK_SOUTH_WEST,
    MASK_EAST_SOUTH_WEST,
    MASK_NORTH_SOUTH_WEST,
];
const CONSTRAINT_EAST_SOUTH: [u8; 3] = [MASK_EAST_SOUTH, MASK_EAST_SOUTH_WEST, MASK_NORTH_EAST_SOUTH];
const CONSTRAINT_NORTH_EAST_WEST: [u8; 8] = [
    MASK_NORTH,
    MASK_NORTH_SOUTH,
    MASK_NORTH_WEST,
    MASK_NORTH_EAST,
    MASK_NORTH_EAST_SOUTH_WEST,
    MASK_NORTH_EAST_WEST,
    MASK_NORTH_SOUTH_WEST,
    MASK_NORTH_EAST_SOUTH,
];
const CONSTRAINT_EAST_SOUTH_WEST: [u8; 8] = [
    MASK_SOUTH,
    MASK_NORTH_SOUTH,
    MASK_SOUTH_WEST,
    MASK_EAST_SOUTH,
    MASK_EAST_SOUTH_WEST,
    MASK_NORTH_SOUTH_WEST,
    MASK_NORTH_EAST_SOUTH,
    MASK_NORTH_EAST_SOUTH_WEST,
];
const CONSTRAINT_NORTH_SOUTH_WEST: [u8; 8] = [
    MASK_WEST,
    MASK_EAST_WEST,
    MASK_NORTH_WEST,
    MASK_SOUTH_WEST,
    MASK_NORTH_EAST_WEST,
    MASK_EAST_SOUTH_WEST,
    MASK_NORTH_SOUTH_WEST,
    MASK_NORTH_EAST_SOUTH_WEST,
];
const CONSTRAINT_NORTH_EAST_SOUTH: [u8; 8] = [
    MASK_EAST,
    MASK_EAST_WEST,
    MASK_NORTH_EAST,
    MASK_EAST_SOUTH,
    MASK_NORTH_EAST_WEST,
    MASK_EAST_SOUTH_WEST,
    MASK_NORTH_EAST_SOUTH,
    MASK_NORTH_EAST_SOUTH_WEST,
];
const CONSTRAINT_NORTH_EAST_SOUTH_WEST: [u8; 12] = [
    MASK_NONE,
    MASK_WEST,
    MASK_EAST,
    MASK_NORTH,
    MASK_SOUTH,
    MASK_NORTH_SOUTH,
    MASK_EAST_WEST,
    MASK_NORTH_EAST_WEST,
    MASK_EAST_SOUTH_WEST,
    MASK_NORTH_SOUTH_WEST,
    MASK_NORTH_EAST_SOUTH,
    MASK_NORTH_EAST_SOUTH_WEST,
];
const CONSTRAINT_EMPTY: [u8; 0] = [];

#[derive(Default)]
pub struct ComponentGroup {
    components: Vec<(u8, Vec<LGraphRef>)>,
}

impl ComponentGroup {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    pub fn with_component(component: LGraphRef) -> Self {
        let mut group = Self::new();
        let _ = group.add(component);
        group
    }

    pub fn add(&mut self, component: LGraphRef) -> bool {
        if !self.can_add(&component) {
            return false;
        }

        let candidate_mask = component_mask(&component);
        if let Some((_, list)) = self.components.iter_mut().find(|(mask, _)| *mask == candidate_mask) {
            list.push(component);
        } else {
            self.components.push((candidate_mask, vec![component]));
        }
        true
    }

    pub fn get_port_sides(&self) -> Vec<EnumSet<PortSide>> {
        let mut sides = Vec::new();
        for (mask, components) in &self.components {
            for _ in components {
                sides.push(mask_to_connections(*mask));
            }
        }
        sides
    }

    pub fn get_components(&self) -> Vec<LGraphRef> {
        let mut all_components = Vec::new();
        for (_, components) in &self.components {
            all_components.extend(components.iter().cloned());
        }
        all_components
    }

    pub fn get_components_for(&self, connections: &EnumSet<PortSide>) -> Vec<LGraphRef> {
        let mask = connections_to_mask(connections);
        self.components
            .iter()
            .find(|(entry_mask, _)| *entry_mask == mask)
            .map(|(_, components)| components.clone())
            .unwrap_or_default()
    }

    fn can_add(&self, component: &LGraphRef) -> bool {
        let candidate_mask = component_mask(component);
        for constraint in constraint_masks(candidate_mask) {
            if self.has_component_with_mask(*constraint) {
                return false;
            }
        }
        true
    }

    fn has_component_with_mask(&self, mask: u8) -> bool {
        self.components
            .iter()
            .any(|(entry_mask, components)| *entry_mask == mask && !components.is_empty())
    }
}

fn component_mask(component: &LGraphRef) -> u8 {
    let connections = component
        .lock()
        .ok()
        .and_then(|mut graph| graph.get_property(InternalProperties::EXT_PORT_CONNECTIONS))
        .unwrap_or_else(EnumSet::none_of);
    connections_to_mask(&connections)
}

fn connections_to_mask(connections: &EnumSet<PortSide>) -> u8 {
    let mut mask = MASK_NONE;
    if connections.contains(&PortSide::North) {
        mask |= MASK_NORTH;
    }
    if connections.contains(&PortSide::East) {
        mask |= MASK_EAST;
    }
    if connections.contains(&PortSide::South) {
        mask |= MASK_SOUTH;
    }
    if connections.contains(&PortSide::West) {
        mask |= MASK_WEST;
    }
    mask
}

fn mask_to_connections(mask: u8) -> EnumSet<PortSide> {
    let mut sides = Vec::new();
    if mask & MASK_NORTH != 0 {
        sides.push(PortSide::North);
    }
    if mask & MASK_EAST != 0 {
        sides.push(PortSide::East);
    }
    if mask & MASK_SOUTH != 0 {
        sides.push(PortSide::South);
    }
    if mask & MASK_WEST != 0 {
        sides.push(PortSide::West);
    }
    EnumSet::of(&sides)
}

fn constraint_masks(candidate_mask: u8) -> &'static [u8] {
    match candidate_mask {
        MASK_NONE => &CONSTRAINT_NONE,
        MASK_WEST => &CONSTRAINT_WEST,
        MASK_EAST => &CONSTRAINT_EAST,
        MASK_NORTH => &CONSTRAINT_NORTH,
        MASK_SOUTH => &CONSTRAINT_SOUTH,
        MASK_NORTH_SOUTH => &CONSTRAINT_NORTH_SOUTH,
        MASK_EAST_WEST => &CONSTRAINT_EAST_WEST,
        MASK_NORTH_WEST => &CONSTRAINT_NORTH_WEST,
        MASK_NORTH_EAST => &CONSTRAINT_NORTH_EAST,
        MASK_SOUTH_WEST => &CONSTRAINT_SOUTH_WEST,
        MASK_EAST_SOUTH => &CONSTRAINT_EAST_SOUTH,
        MASK_NORTH_EAST_WEST => &CONSTRAINT_NORTH_EAST_WEST,
        MASK_EAST_SOUTH_WEST => &CONSTRAINT_EAST_SOUTH_WEST,
        MASK_NORTH_SOUTH_WEST => &CONSTRAINT_NORTH_SOUTH_WEST,
        MASK_NORTH_EAST_SOUTH => &CONSTRAINT_NORTH_EAST_SOUTH,
        MASK_NORTH_EAST_SOUTH_WEST => &CONSTRAINT_NORTH_EAST_SOUTH_WEST,
        _ => &CONSTRAINT_EMPTY,
    }
}
