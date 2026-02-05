use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::topdownpacking::grid::Grid;
use crate::org::eclipse::elk::alg::topdownpacking::grid_elk_node::GridElkNode;
use crate::org::eclipse::elk::alg::topdownpacking::i_node_arranger::INodeArranger;
use crate::org::eclipse::elk::alg::topdownpacking::options::TopdownpackingOptions;
use crate::org::eclipse::elk::alg::topdownpacking::topdown_packing_phases::TopdownPackingPhases;

pub struct LeftRightTopDownNodePlacer;

impl LeftRightTopDownNodePlacer {
    pub fn new() -> Self {
        LeftRightTopDownNodePlacer
    }
}

impl Default for LeftRightTopDownNodePlacer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<TopdownPackingPhases, GridElkNode> for LeftRightTopDownNodePlacer {
    fn process(&mut self, layout_graph: &mut GridElkNode, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Node placement", 1.0);
        progress_monitor.log(&format!(
            "Node placement began for node {}",
            layout_graph.identifier()
        ));

        let padding = layout_graph
            .get_property(TopdownpackingOptions::PADDING)
            .unwrap_or_default();
        let node_node_spacing = layout_graph
            .get_property(TopdownpackingOptions::SPACING_NODE_NODE)
            .unwrap_or(0.0);

        let graph_size = self.get_predicted_size(layout_graph.node());
        layout_graph.set_dimensions(
            layout_graph.width().max(graph_size.x),
            layout_graph.height().max(graph_size.y),
        );

        let nodes = layout_graph.children();

        let cols = (nodes.len() as f64).sqrt().ceil() as usize;
        let rows = if nodes.len() > cols * cols - cols || cols == 0 {
            cols
        } else {
            cols - 1
        };

        layout_graph.set_grid_size(cols, rows);

        progress_monitor.log(&format!(
            "{}\nPlacing {} nodes in {} columns.",
            layout_graph.identifier(),
            nodes.len(),
            cols
        ));
        progress_monitor.done();
        progress_monitor.log("Node Arrangement done!");

        let mut curr_x = padding.left;
        let mut curr_y = padding.top;
        let mut current_col = 0usize;
        let mut current_row = 0usize;

        let desired_node_width = layout_graph
            .get_property(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH)
            .unwrap_or(0.0);
        let aspect_ratio = layout_graph
            .get_property(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO)
            .unwrap_or(1.0);

        for node in nodes {
            set_node_dimensions(&node, desired_node_width, desired_node_width / aspect_ratio);
            set_node_x(&node, curr_x);
            set_node_y(&node, curr_y);

            progress_monitor.log(&format!("currX: {}", curr_x));
            progress_monitor.log(&format!("currY: {}", curr_y));

            layout_graph.put(current_col, current_row, node.clone());

            progress_monitor.log_graph(
                layout_graph.node(),
                &format!("{} placed in ({}|{})", node_identifier(&node), current_col, current_row),
            );

            curr_x += node_width(&node) + node_node_spacing;
            current_col += 1;

            if current_col >= cols {
                curr_x = padding.left;
                curr_y += desired_node_width / aspect_ratio + node_node_spacing;
                current_col = 0;
                current_row += 1;
            }
        }

        progress_monitor.log("Node Placing done!");
        progress_monitor.log_graph(layout_graph.node(), "Graph after node placement");
        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &GridElkNode,
    ) -> Option<LayoutProcessorConfiguration<TopdownPackingPhases, GridElkNode>> {
        Some(LayoutProcessorConfiguration::create())
    }
}

impl INodeArranger for LeftRightTopDownNodePlacer {
    fn get_predicted_size(&self, graph: &ElkNodeRef) -> KVector {
        let number_of_children = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().len()
        };

        let padding = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(TopdownpackingOptions::PADDING)
        }
        .unwrap_or_default();
        let node_node_spacing = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(TopdownpackingOptions::SPACING_NODE_NODE)
        }
        .unwrap_or(0.0);
        let hierarchical_node_width = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH)
        }
        .unwrap_or(0.0);
        let hierarchical_node_aspect_ratio = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO)
        }
        .unwrap_or(1.0);

        let cols = (number_of_children as f64).sqrt().ceil() as i32;
        let required_width = cols as f64 * hierarchical_node_width
            + padding.left
            + padding.right
            + (cols - 1) as f64 * node_node_spacing;

        let rows = if number_of_children > (cols * cols - cols) as usize || cols == 0 {
            cols
        } else {
            cols - 1
        };

        let required_height = rows as f64 * hierarchical_node_width / hierarchical_node_aspect_ratio
            + padding.top
            + padding.bottom
            + (rows - 1) as f64 * node_node_spacing;

        KVector::with_values(required_width, required_height)
    }
}

fn node_width(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().width()
}

fn node_identifier(node: &ElkNodeRef) -> String {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .identifier()
        .unwrap_or("")
        .to_string()
}

fn set_node_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn set_node_x(node: &ElkNodeRef, x: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_x(x);
}

fn set_node_y(node: &ElkNodeRef, y: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_y(y);
}
