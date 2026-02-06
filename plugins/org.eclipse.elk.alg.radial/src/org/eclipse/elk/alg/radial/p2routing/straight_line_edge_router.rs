use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkMath;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkEdgeSectionRef, ElkGraphFactory, ElkNodeRef,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

use crate::org::eclipse::elk::alg::radial::p2routing::IRadialEdgeRouter;
use crate::org::eclipse::elk::alg::radial::radial_layout_phases::RadialLayoutPhases;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;

pub struct StraightLineEdgeRouter;

impl StraightLineEdgeRouter {
    pub fn new() -> Self {
        StraightLineEdgeRouter
    }

    fn route_edges_internal(node: &ElkNodeRef) {
        for edge in ElkGraphUtil::all_outgoing_edges(node) {
            let (source_shape, target_shape) = {
                let edge_borrow = edge.borrow();
                (
                    edge_borrow.sources_ro().get(0),
                    edge_borrow.targets_ro().get(0),
                )
            };

            let Some(source_shape) = source_shape else { continue; };
            if matches!(source_shape, ElkConnectableShapeRef::Port(_)) {
                continue;
            }

            let Some(target_shape) = target_shape else { continue; };
            let Some(target) = ElkGraphUtil::connectable_shape_to_node(&target_shape) else {
                continue;
            };

            let hierarchical = edge.borrow().is_hierarchical();
            if hierarchical {
                continue;
            }

            let (mut source_x, mut source_y) = node_center(node);
            let (mut target_x, mut target_y) = node_center(&target);

            let mut vector = KVector::new();
            vector.x = target_x - source_x;
            vector.y = target_y - source_y;

            let (node_width, node_height) = node_size(node);
            let mut source_clip = KVector::with_values(vector.x, vector.y);
            ElkMath::clip_vector(&mut source_clip, node_width, node_height);
            vector.x -= source_clip.x;
            vector.y -= source_clip.y;

            source_x = target_x - vector.x;
            source_y = target_y - vector.y;

            let (target_width, target_height) = node_size(&target);
            let mut target_clip = KVector::with_values(vector.x, vector.y);
            ElkMath::clip_vector(&mut target_clip, target_width, target_height);
            vector.x -= target_clip.x;
            vector.y -= target_clip.y;

            target_x = source_x + vector.x;
            target_y = source_y + vector.y;

            if let Some(section) = first_edge_section(&edge, true, true) {
                let mut section_mut = section.borrow_mut();
                section_mut.set_start_x(source_x);
                section_mut.set_start_y(source_y);
                section_mut.set_end_x(target_x);
                section_mut.set_end_y(target_y);
            }

            Self::route_edges_internal(&target);
        }
    }
}

impl Default for StraightLineEdgeRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl IRadialEdgeRouter for StraightLineEdgeRouter {
    fn route_edges(&mut self, node: &ElkNodeRef) {
        Self::route_edges_internal(node);
    }
}

impl ILayoutPhase<RadialLayoutPhases, ElkNodeRef> for StraightLineEdgeRouter {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Straight Line Edge Routing", 1.0);
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "Before");
        }
        if let Some(root) = RadialUtil::root_from_graph(graph) {
            Self::route_edges_internal(&root);
        }
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "After");
        }
        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &ElkNodeRef,
    ) -> Option<LayoutProcessorConfiguration<RadialLayoutPhases, ElkNodeRef>> {
        None
    }
}

fn node_center(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    let x = shape.x() + shape.width() / 2.0;
    let y = shape.y() + shape.height() / 2.0;
    (x, y)
}

fn node_size(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
}

fn first_edge_section(
    edge: &ElkEdgeRef,
    reset_section: bool,
    remove_other_sections: bool,
) -> Option<ElkEdgeSectionRef> {
    let mut edge_mut = edge.borrow_mut();
    let sections = edge_mut.sections();
    if sections.is_empty() {
        let section = ElkGraphFactory::instance().create_elk_edge_section();
        sections.add(section.clone());
        return Some(section);
    }

    let section = sections.get(0);
    if let Some(section_ref) = section.as_ref() {
        if reset_section {
            let mut section_mut = section_ref.borrow_mut();
            section_mut.bend_points().clear();
            section_mut.set_start_x(0.0);
            section_mut.set_start_y(0.0);
            section_mut.set_end_x(0.0);
            section_mut.set_end_y(0.0);
        }
        if remove_other_sections && sections.len() > 1 {
            let keep = section_ref.clone();
            sections.clear();
            sections.add(keep);
        }
    }
    section
}
