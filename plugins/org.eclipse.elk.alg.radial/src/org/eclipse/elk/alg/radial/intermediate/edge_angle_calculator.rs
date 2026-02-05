use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};

use crate::org::eclipse::elk::alg::radial::options::RadialOptions;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;

#[derive(Default)]
pub struct EdgeAngleCalculator;

impl ILayoutProcessor<ElkNodeRef> for EdgeAngleCalculator {
    fn process(&mut self, graph: &mut ElkNodeRef, _progress_monitor: &mut dyn IElkProgressMonitor) {
        let root = RadialUtil::root_from_graph(graph);
        let Some(root) = root else { return; };

        let outgoing_edges = {
            let mut root_mut = root.borrow_mut();
            root_mut.connectable().outgoing_edges().iter().collect::<Vec<_>>()
        };
        for edge in outgoing_edges {
            let section = {
                let mut edge_borrow = edge.borrow_mut();
                edge_borrow.sections().get(0)
            };
            let Some(section) = section else { continue; };
            let (start, end) = {
                let section_borrow = section.borrow();
                (
                    KVector::with_values(section_borrow.start_x(), section_borrow.start_y()),
                    KVector::with_values(section_borrow.end_x(), section_borrow.end_y()),
                )
            };
            let edge_vector = KVector::from_points(&start, &end);
            let angle = edge_vector.y.atan2(edge_vector.x);

            let target_shape = {
                let edge_borrow = edge.borrow();
                edge_borrow.targets_ro().get(0)
            };
            let Some(target_shape) = target_shape else { continue; };
            match target_shape {
                ElkConnectableShapeRef::Node(node) => {
                    let mut node_mut = node.borrow_mut();
                    node_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .set_property(RadialOptions::ROTATION_TARGET_ANGLE, Some(angle));
                }
                ElkConnectableShapeRef::Port(port) => {
                    let mut port_mut = port.borrow_mut();
                    port_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .set_property(RadialOptions::ROTATION_TARGET_ANGLE, Some(angle));
                }
            }
        }
    }
}
