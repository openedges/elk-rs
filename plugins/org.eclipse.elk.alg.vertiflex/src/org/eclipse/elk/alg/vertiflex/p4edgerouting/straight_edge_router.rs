use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeRef;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeSectionRef;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphFactory;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

use crate::org::eclipse::elk::alg::vertiflex::vertiflex_layout_phases::VertiFlexLayoutPhases;
use crate::org::eclipse::elk::alg::vertiflex::vertiflex_util::VertiFlexUtil;

pub struct StraightEdgeRouter;

impl StraightEdgeRouter {
    pub fn new() -> Self {
        StraightEdgeRouter
    }

    fn route_edges(&self, node: &ElkNodeRef) {
        for edge in ElkGraphUtil::all_outgoing_edges(node) {
            let Some(target) = edge_target_node(&edge) else { continue; };
            let Some(section) = ensure_single_section(&edge) else { continue; };

            let start_x = node_x(node) + node_width(node) / 2.0;
            let start_y = node_y(node) + node_height(node);
            let end_x = node_x(&target) + node_width(&target) / 2.0;
            let end_y = node_y(&target);

            {
                let mut section_mut = section.borrow_mut();
                section_mut.set_start_x(start_x);
                section_mut.set_start_y(start_y);
                section_mut.set_end_x(end_x);
                section_mut.set_end_y(end_y);
                section_mut.bend_points().clear();
            }

            self.route_edges(&target);
        }
    }
}

impl Default for StraightEdgeRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<VertiFlexLayoutPhases, ElkNodeRef> for StraightEdgeRouter {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("StraightEdgeRouter", 1.0);

        let has_children = {
            let mut graph_mut = graph.borrow_mut();
            !graph_mut.children().is_empty()
        };
        if has_children {
            if let Some(parent) = VertiFlexUtil::find_root(graph) {
                self.route_edges(&parent);
            }
        }

        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &ElkNodeRef,
    ) -> Option<LayoutProcessorConfiguration<VertiFlexLayoutPhases, ElkNodeRef>> {
        None
    }
}

fn ensure_single_section(edge: &ElkEdgeRef) -> Option<ElkEdgeSectionRef> {
    let mut edge_mut = edge.borrow_mut();
    let sections = edge_mut.sections();
    if sections.is_empty() {
        let section = ElkGraphFactory::instance().create_elk_edge_section();
        sections.add(section.clone());
        return Some(section);
    }
    if sections.len() > 1 {
        sections.retain_last();
    }
    sections.get(0)
}

fn edge_target_node(edge: &ElkEdgeRef) -> Option<ElkNodeRef> {
    let edge_borrow = edge.borrow();
    let target = edge_borrow.targets_ro().get(0)?;
    drop(edge_borrow);
    ElkGraphUtil::connectable_shape_to_node(&target)
}

fn node_x(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().x()
}

fn node_y(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().y()
}

fn node_width(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().width()
}

fn node_height(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().height()
}
