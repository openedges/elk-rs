use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::topdownpacking::grid::Grid;
use crate::org::eclipse::elk::alg::topdownpacking::grid_elk_node::GridElkNode;
use crate::org::eclipse::elk::alg::topdownpacking::topdown_packing_phases::TopdownPackingPhases;

pub struct BottomRowEqualWhitespaceEliminator;

impl BottomRowEqualWhitespaceEliminator {
    pub fn new() -> Self {
        BottomRowEqualWhitespaceEliminator
    }
}

impl Default for BottomRowEqualWhitespaceEliminator {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<TopdownPackingPhases, GridElkNode> for BottomRowEqualWhitespaceEliminator {
    fn process(
        &mut self,
        layout_graph: &mut GridElkNode,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        progress_monitor.begin("Whitespace elimination", 1.0);
        progress_monitor.log(&format!(
            "Whitespace elimination began for node {}",
            layout_graph.identifier()
        ));

        if layout_graph.width() == 0.0 || layout_graph.columns() == 0 {
            progress_monitor.log("Parent node has no width, skipping phase");
            progress_monitor.done();
            return;
        }

        let padding = layout_graph
            .get_property(CoreOptions::PADDING)
            .unwrap_or_default();

        for row_index in 0..layout_graph.rows() {
            let row = layout_graph.get_row(row_index);
            if row.is_empty() {
                continue;
            }
            let mut last_index = row.len();
            let mut last: Option<ElkNodeRef> = None;
            while last.is_none() {
                if last_index == 0 {
                    break;
                }
                last_index -= 1;
                last = row[last_index].clone();
            }
            let Some(last_node) = last else { continue };

            let right_border = node_x(&last_node) + node_width(&last_node);
            if right_border + padding.right < layout_graph.width() {
                progress_monitor.log(&format!("Eliminate white space in row {}", row_index));
                let extra_space = layout_graph.width() - (right_border + padding.right);
                let extra_space_per_node = extra_space / (last_index as f64 + 1.0);
                let mut accumulated_shift = 0.0;
                for item in row.iter().take(last_index + 1) {
                    if let Some(node) = item.clone() {
                        set_node_x(&node, node_x(&node) + accumulated_shift);
                        set_node_width(&node, node_width(&node) + extra_space_per_node);
                    }
                    accumulated_shift += extra_space_per_node;
                }
            }
        }

        let col = layout_graph.get_column(0);
        let last: Option<ElkNodeRef> = col.iter().rev().flatten().next().cloned();
        let Some(last_node) = last else {
            progress_monitor.log_graph(layout_graph.node(), "Graph after whitespace elimination");
            progress_monitor.done();
            return;
        };

        let bottom_border = node_y(&last_node) + node_height(&last_node);
        let extra_space = layout_graph.height() - (bottom_border + padding.bottom);
        let extra_space_per_node = extra_space / (col.len() as f64 + 1.0);
        let mut accumulated_shift = 0.0;
        progress_monitor.log("Eliminate vertical white space");
        if bottom_border + padding.bottom < layout_graph.height() {
            for row_index in 0..layout_graph.rows() {
                let row = layout_graph.get_row(row_index);
                for item in row {
                    let Some(node) = item else { break };
                    set_node_y(&node, node_y(&node) + accumulated_shift);
                    set_node_height(&node, node_height(&node) + extra_space_per_node);
                    accumulated_shift += extra_space_per_node;
                }
            }
        }

        progress_monitor.log_graph(layout_graph.node(), "Graph after whitespace elimination");
        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &GridElkNode,
    ) -> Option<LayoutProcessorConfiguration<TopdownPackingPhases, GridElkNode>> {
        Some(LayoutProcessorConfiguration::create())
    }
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

fn set_node_x(node: &ElkNodeRef, x: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_x(x);
}

fn set_node_y(node: &ElkNodeRef, y: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_y(y);
}

fn set_node_width(node: &ElkNodeRef, width: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_width(width);
}

fn set_node_height(node: &ElkNodeRef, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_height(height);
}
