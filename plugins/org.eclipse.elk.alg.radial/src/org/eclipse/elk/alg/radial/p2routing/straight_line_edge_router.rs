use std::collections::HashMap;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkMath;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkEdgeSectionRef, ElkGraphFactory, ElkNodeRef,
};

use crate::org::eclipse::elk::alg::radial::p2routing::IRadialEdgeRouter;
use crate::org::eclipse::elk::alg::radial::radial_layout_phases::RadialLayoutPhases;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;

/// Pre-extracted node geometry: (center_x, center_y, width, height)
type NodeGeom = (f64, f64, f64, f64);

pub struct StraightLineEdgeRouter;

impl StraightLineEdgeRouter {
    pub fn new() -> Self {
        StraightLineEdgeRouter
    }

    /// Pre-extract center and size for all graph children in a single pass.
    fn build_node_geom(graph: &ElkNodeRef) -> HashMap<usize, NodeGeom> {
        let children: Vec<ElkNodeRef> = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().iter().cloned().collect()
        };
        let mut geom = HashMap::with_capacity(children.len());
        for child in &children {
            let mut node_mut = child.borrow_mut();
            let shape = node_mut.connectable().shape();
            let w = shape.width();
            let h = shape.height();
            let cx = shape.x() + w / 2.0;
            let cy = shape.y() + h / 2.0;
            geom.insert(Rc::as_ptr(child) as usize, (cx, cy, w, h));
        }
        geom
    }

    fn route_edges_cached(node: &ElkNodeRef, geom: &HashMap<usize, NodeGeom>) {
        let node_key = Rc::as_ptr(node) as usize;
        for edge in ElkGraphUtil::all_outgoing_edges(node) {
            let (source_shape, target_shape) = {
                let edge_borrow = edge.borrow();
                (
                    edge_borrow.sources_ro().get(0),
                    edge_borrow.targets_ro().get(0),
                )
            };

            let Some(source_shape) = source_shape else {
                continue;
            };
            if matches!(source_shape, ElkConnectableShapeRef::Port(_)) {
                continue;
            }

            let Some(target_shape) = target_shape else {
                continue;
            };
            let Some(target) = ElkGraphUtil::connectable_shape_to_node(&target_shape) else {
                continue;
            };

            let hierarchical = edge.borrow().is_hierarchical();
            if hierarchical {
                continue;
            }

            let target_key = Rc::as_ptr(&target) as usize;

            // Use cached geometry — zero borrows for center/size
            let (mut source_x, mut source_y, node_width, node_height) =
                geom.get(&node_key).copied().unwrap_or_else(|| node_geom(node));
            let (mut target_x, mut target_y, target_width, target_height) =
                geom.get(&target_key).copied().unwrap_or_else(|| node_geom(&target));

            let mut vector = KVector::new();
            vector.x = target_x - source_x;
            vector.y = target_y - source_y;

            let mut source_clip = KVector::with_values(vector.x, vector.y);
            ElkMath::clip_vector(&mut source_clip, node_width, node_height);
            vector.x -= source_clip.x;
            vector.y -= source_clip.y;

            source_x = target_x - vector.x;
            source_y = target_y - vector.y;

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

            Self::route_edges_cached(&target, geom);
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
        // Fallback path without pre-built cache — build one from node's parent
        let geom = HashMap::new();
        Self::route_edges_cached(node, &geom);
    }
}

impl ILayoutPhase<RadialLayoutPhases, ElkNodeRef> for StraightLineEdgeRouter {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Straight Line Edge Routing", 1.0);
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "Before");
        }
        if let Some(root) = RadialUtil::root_from_graph(graph) {
            let geom = Self::build_node_geom(graph);
            Self::route_edges_cached(&root, &geom);
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

/// Fallback: extract geometry from a single node (used when cache miss).
fn node_geom(node: &ElkNodeRef) -> NodeGeom {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    let w = shape.width();
    let h = shape.height();
    (shape.x() + w / 2.0, shape.y() + h / 2.0, w, h)
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
