use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::compaction::AbstractRadiusExtensionCompaction;
use crate::org::eclipse::elk::alg::radial::intermediate::overlaps::IOverlapRemoval;
use crate::org::eclipse::elk::alg::radial::options::RadialOptions;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;
use crate::org::eclipse::elk::alg::radial::sorting::IRadialSorter;

#[derive(Default)]
pub struct RadiusExtensionOverlapRemoval {
    base: AbstractRadiusExtensionCompaction,
    sorter: Option<Box<dyn IRadialSorter>>,
}

impl RadiusExtensionOverlapRemoval {
    fn extend(
        &mut self,
        graph: &ElkNodeRef,
        root: &ElkNodeRef,
        nodes: Vec<ElkNodeRef>,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        if nodes.is_empty() {
            return;
        }

        let mut old_positions = Vec::new();
        for node in &nodes {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            old_positions.push(KVector::with_values(shape.x(), shape.y()));
        }

        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "Before removing overlaps");
        }
        let mut iterations = 0usize;
        while self.base.overlap_layer(&nodes) {
            if iterations >= 10_000 {
                break;
            }
            self.base.contract_layer(root, &nodes, false);
            iterations += 1;
        }
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "After removing overlaps");
        }

        let mut moved_distance = 0.0;
        if let Some(first) = nodes.first() {
            let (new_x, new_y) = {
                let mut node_mut = first.borrow_mut();
                let shape = node_mut.connectable().shape();
                (shape.x(), shape.y())
            };
            let moved_x = new_x - old_positions[0].x;
            let moved_y = new_y - old_positions[0].y;
            moved_distance = (moved_x * moved_x + moved_y * moved_y).sqrt();
        }

        let next_level_nodes = RadialUtil::get_next_level_node_set(&nodes);
        if !next_level_nodes.is_empty() {
            for next_level_node in &next_level_nodes {
                self.base.move_node(root, next_level_node, moved_distance);
            }
            if progress_monitor.is_logging_enabled() {
                progress_monitor.log_graph(graph, "Child movement 1");
            }
        }

        let mut next_level_nodes = next_level_nodes;
        if let Some(sorter) = self.sorter.as_mut() {
            sorter.sort(&mut next_level_nodes);
        }
        self.extend(graph, root, next_level_nodes, progress_monitor);
    }
}

impl IOverlapRemoval for RadiusExtensionOverlapRemoval {
    fn remove_overlaps(
        &mut self,
        graph: &ElkNodeRef,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        let root = RadialUtil::root_from_graph(graph);
        let Some(root) = root else {
            return;
        };
        self.sorter = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RadialOptions::SORTER)
        }
        .unwrap_or_default()
        .create();

        let spacing = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::SPACING_NODE_NODE)
        }
        .unwrap_or(0.0);
        self.base.set_spacing(spacing);

        let successors = RadialUtil::get_successors(&root);
        self.extend(graph, &root, successors, progress_monitor);
    }
}

impl ILayoutProcessor<ElkNodeRef> for RadiusExtensionOverlapRemoval {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Remove overlaps", 1.0);
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "Before");
        }
        self.remove_overlaps(graph, progress_monitor);
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "After");
        }
    }
}
