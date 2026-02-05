use std::collections::HashMap;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::naive_min_st::NaiveMinST;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::internal_properties::InternalProperties;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::node::Node;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::tree::Tree;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, IElkProgressMonitor};

use crate::org::eclipse::elk::alg::spore::graph::Graph;
use crate::org::eclipse::elk::alg::spore::p2processingorder::cost_function::GraphCostFunction;
use crate::org::eclipse::elk::alg::spore::spore_phases::SPOrEPhases;

pub struct MinSTPhase {
    node_map: HashMap<KVector, Node>,
}

impl MinSTPhase {
    pub fn new() -> Self {
        MinSTPhase {
            node_map: HashMap::new(),
        }
    }

    pub(crate) fn convert_tree(&mut self, t_tree: &Tree<KVector>, graph: &mut Graph) {
        self.node_map.clear();
        for node in &graph.vertices {
            self.node_map.insert(node.original_vertex, node.clone());
        }

        let Some(root_node) = self.node_map.get(&t_tree.node).cloned() else {
            graph.tree = None;
            return;
        };

        let mut root = Tree::new(root_node);
        add_node(&mut root, t_tree, &self.node_map);
        graph.tree = Some(root);
    }
}

impl Default for MinSTPhase {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<SPOrEPhases, Graph> for MinSTPhase {
    fn process(&mut self, graph: &mut Graph, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Minimum spanning tree construction", 1.0);

        if graph.vertices.is_empty() {
            progress_monitor.done();
            return;
        }

        let root = graph
            .preferred_root()
            .map(|node| node.original_vertex)
            .unwrap_or(graph.vertices[0].original_vertex);

        let debug_output = graph
            .get_property(InternalProperties::DEBUG_SVG)
            .unwrap_or(false)
            .then(|| ElkUtil::debug_folder_path(&["spore"]))
            .flatten()
            .map(|path| format!("{}20minst", path));

        let Some(edges) = graph.t_edges.as_ref() else {
            progress_monitor.done();
            return;
        };

        let cost_function = GraphCostFunction::new(graph);

        let tree = NaiveMinST::create_spanning_tree(edges, &root, &cost_function, debug_output.as_deref());
        self.convert_tree(&tree, graph);

        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &Graph,
    ) -> Option<LayoutProcessorConfiguration<SPOrEPhases, Graph>> {
        Some(LayoutProcessorConfiguration::create())
    }
}

fn add_node(root: &mut Tree<Node>, t_tree: &Tree<KVector>, node_map: &HashMap<KVector, Node>) {
    for child in &t_tree.children {
        if let Some(node) = node_map.get(&child.node) {
            let mut child_tree = Tree::new(node.clone());
            add_node(&mut child_tree, child, node_map);
            root.children.push(child_tree);
        }
    }
}
