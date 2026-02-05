use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::compaction::{
    AbstractRadiusExtensionCompaction, IRadialCompactor,
};
use crate::org::eclipse::elk::alg::radial::options::RadialOptions;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;
use crate::org::eclipse::elk::alg::radial::sorting::IRadialSorter;

#[derive(Default)]
pub struct RadialCompaction {
    base: AbstractRadiusExtensionCompaction,
    sorter: Option<Box<dyn IRadialSorter>>,
    last_radius: f64,
}

impl RadialCompaction {
    pub fn new() -> Self {
        RadialCompaction {
            base: AbstractRadiusExtensionCompaction::new(),
            sorter: None,
            last_radius: 0.0,
        }
    }

    fn contract(&mut self, root: &ElkNodeRef, nodes: Vec<ElkNodeRef>) {
        if nodes.is_empty() {
            return;
        }

        let mut is_overlapping = self.overlapping(root, &nodes);
        let mut was_contracted = false;
        while !is_overlapping {
            self.base.contract_layer(root, &nodes, true);
            was_contracted = true;
            is_overlapping = self.overlapping(root, &nodes);
        }

        if was_contracted {
            self.base.contract_layer(root, &nodes, false);
        }

        let mut next_level_nodes = RadialUtil::get_next_level_nodes(&nodes);
        if let Some(sorter) = self.sorter.as_mut() {
            sorter.sort(&mut next_level_nodes);
        }
        self.last_radius = self.calculate_radius(root, &nodes[0]);
        self.contract(root, next_level_nodes);
    }

    fn calculate_radius(&self, root: &ElkNodeRef, node: &ElkNodeRef) -> f64 {
        let (x_pos, y_pos) = node_center(node);
        let (root_x, root_y) = node_center(root);
        let vector_x = x_pos - root_x;
        let vector_y = y_pos - root_y;
        (vector_x * vector_x + vector_y * vector_y).sqrt()
    }

    fn overlapping(&self, root: &ElkNodeRef, nodes: &[ElkNodeRef]) -> bool {
        if self.base.overlap_layer(nodes) {
            return true;
        }
        for node in nodes {
            if let Some(parent) = RadialUtil::get_tree_parent(node) {
                if self.base.overlap(node, &parent) {
                    return true;
                }
            }
            if self.calculate_radius(root, node) - self.base.get_spacing() <= self.last_radius {
                return true;
            }
        }
        false
    }
}

impl IRadialCompactor for RadialCompaction {
    fn compact(&mut self, graph: &ElkNodeRef) {
        let root = RadialUtil::root_from_graph(graph);
        let Some(root) = root else { return; };

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

        let step_size = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RadialOptions::COMPACTION_STEP_SIZE)
        };
        if let Some(step_size) = step_size {
            self.base.set_compaction_step(step_size);
        }

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

        let mut first_level_nodes = RadialUtil::get_successors(&root);
        if let Some(sorter) = self.sorter.as_mut() {
            sorter.sort(&mut first_level_nodes);
        }
        self.contract(&root, first_level_nodes);
    }
}

fn node_center(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (
        shape.x() + shape.width() / 2.0,
        shape.y() + shape.height() / 2.0,
    )
}
