use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::vertiflex::vertiflex_layout_phases::VertiFlexLayoutPhases;
use crate::org::eclipse::elk::alg::vertiflex::vertiflex_util::VertiFlexUtil;

pub struct AbsoluteXPlacer;

impl AbsoluteXPlacer {
    pub fn new() -> Self {
        AbsoluteXPlacer
    }

    fn find_minimal_x(&self, tree: &ElkNodeRef) -> f64 {
        let children = outgoing_children(tree);
        if children.is_empty() {
            node_x(tree)
        } else {
            let mut min_subtree_x = 0.0;
            for child in children {
                let test_x = self.find_minimal_x(&child);
                if test_x < min_subtree_x {
                    min_subtree_x = test_x;
                }
            }
            min_subtree_x + node_x(tree)
        }
    }

    fn absolute_tree_coords(&self, tree: &ElkNodeRef) {
        let children = outgoing_children(tree);
        if !children.is_empty() {
            let base_x = node_x(tree);
            for child in children {
                node_set_x(&child, node_x(&child) + base_x);
                self.absolute_tree_coords(&child);
            }
        }
    }
}

impl Default for AbsoluteXPlacer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<VertiFlexLayoutPhases, ElkNodeRef> for AbsoluteXPlacer {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("AbsolutPlacer", 1.0);

        let has_children = {
            let mut graph_mut = graph.borrow_mut();
            !graph_mut.children().is_empty()
        };
        if has_children {
            if let Some(parent) = VertiFlexUtil::find_root(graph) {
                let new_x = node_x(&parent) - self.find_minimal_x(&parent);
                node_set_x(&parent, new_x);
                self.absolute_tree_coords(&parent);
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

fn outgoing_children(node: &ElkNodeRef) -> Vec<ElkNodeRef> {
    let mut children = Vec::new();
    for edge in ElkGraphUtil::all_outgoing_edges(node) {
        if let Some(child) = edge_target_node(&edge) {
            children.push(child);
        }
    }
    children
}

fn edge_target_node(edge: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeRef) -> Option<ElkNodeRef> {
    let edge_borrow = edge.borrow();
    let target = edge_borrow.targets_ro().get(0)?;
    drop(edge_borrow);
    ElkGraphUtil::connectable_shape_to_node(&target)
}

fn node_x(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().x()
}

fn node_set_x(node: &ElkNodeRef, value: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_x(value);
}
