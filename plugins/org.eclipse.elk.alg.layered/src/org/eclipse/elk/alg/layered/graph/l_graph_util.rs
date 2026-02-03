use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, PortType};

use super::{LGraphRef, LNodeRef, LPort, LPortRef};

pub struct LGraphUtil;

impl LGraphUtil {
    pub fn to_node_array(nodes: &[LNodeRef]) -> Vec<LNodeRef> {
        nodes.to_vec()
    }

    pub fn to_edge_array(edges: &[super::LEdgeRef]) -> Vec<super::LEdgeRef> {
        edges.to_vec()
    }

    pub fn to_port_array(ports: &[LPortRef]) -> Vec<LPortRef> {
        ports.to_vec()
    }

    pub fn provide_collector_port(
        _layered_graph: &LGraphRef,
        node: &LNodeRef,
        port_type: PortType,
        side: PortSide,
    ) -> LPortRef {
        let mut port: Option<LPortRef> = None;

        match port_type {
            PortType::Input => {
                if let Ok(node_guard) = node.lock() {
                    for candidate in node_guard.ports() {
                        if candidate
                            .lock()
                            .ok()
                            .and_then(|mut port| port.get_property(InternalProperties::INPUT_COLLECT))
                            .unwrap_or(false)
                        {
                            return candidate.clone();
                        }
                    }
                }
                let created = LPort::new();
                created
                    .lock()
                    .ok()
                    .map(|mut port| port.set_property(InternalProperties::INPUT_COLLECT, Some(true)));
                port = Some(created);
            }
            PortType::Output => {
                if let Ok(node_guard) = node.lock() {
                    for candidate in node_guard.ports() {
                        if candidate
                            .lock()
                            .ok()
                            .and_then(|mut port| port.get_property(InternalProperties::OUTPUT_COLLECT))
                            .unwrap_or(false)
                        {
                            return candidate.clone();
                        }
                    }
                }
                let created = LPort::new();
                created
                    .lock()
                    .ok()
                    .map(|mut port| port.set_property(InternalProperties::OUTPUT_COLLECT, Some(true)));
                port = Some(created);
            }
            PortType::Undefined => {}
        }

        if let Some(port_ref) = port {
            LPort::set_node(&port_ref, Some(node.clone()));
            if let Ok(mut port_guard) = port_ref.lock() {
                port_guard.set_side(side);
                let size = node
                    .lock()
                    .map(|mut node| *node.shape().size_ref())
                    .unwrap_or(KVector::new());
                let mut pos = KVector::new();
                LGraphUtil::center_point(&mut pos, &size, side);
                *port_guard.shape().position() = pos;
            }
            return port_ref;
        }

        LPort::new()
    }

    fn center_point(point: &mut KVector, boundary: &KVector, side: PortSide) {
        match side {
            PortSide::North => {
                point.x = boundary.x / 2.0;
                point.y = 0.0;
            }
            PortSide::East => {
                point.x = boundary.x;
                point.y = boundary.y / 2.0;
            }
            PortSide::South => {
                point.x = boundary.x / 2.0;
                point.y = boundary.y;
            }
            PortSide::West => {
                point.x = 0.0;
                point.y = boundary.y / 2.0;
            }
            PortSide::Undefined => {}
        }
    }
}
