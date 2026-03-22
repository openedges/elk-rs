use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef, Layer, LayerRef,
};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

pub struct HighDegreeNodeLayeringProcessor {
    degree_threshold: i32,
    tree_height_threshold: i32,
}

impl Default for HighDegreeNodeLayeringProcessor {
    fn default() -> Self {
        Self {
            degree_threshold: 16,
            tree_height_threshold: 5,
        }
    }
}

impl ILayoutProcessor<LGraph> for HighDegreeNodeLayeringProcessor {
    fn process(
        &mut self,
        layered_graph: &mut LGraph,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        progress_monitor.begin("High degree node processing", 1.0);

        self.degree_threshold = layered_graph
            .get_property(LayeredOptions::HIGH_DEGREE_NODES_THRESHOLD)
            .unwrap_or(16);
        self.tree_height_threshold = layered_graph
            .get_property(LayeredOptions::HIGH_DEGREE_NODES_TREE_HEIGHT)
            .unwrap_or(5);
        if self.tree_height_threshold == 0 {
            self.tree_height_threshold = i32::MAX;
        }

        let graph_ref = graph_ref_for(layered_graph);
        let original_layers = layered_graph.layers().clone();

        for layer in original_layers {
            let Some(layer_index) = index_of_layer(layered_graph.layers(), &layer) else {
                continue;
            };

            let (high_degree_nodes, inc_max, out_max) = self.collect_high_degree_nodes(&layer);

            let mut pre_layers: Vec<LayerRef> = Vec::new();
            if inc_max > 0 {
                for offset in 0..inc_max as usize {
                    let new_layer = Layer::new(&graph_ref);
                    layered_graph
                        .layers_mut()
                        .insert(layer_index + offset, new_layer.clone());
                    pre_layers.insert(0, new_layer);
                }
            }
            if !pre_layers.is_empty() {
                for (_, info) in &high_degree_nodes {
                    for root in &info.inc_tree_roots {
                        move_tree(root, EdgeSelector::Incoming, &pre_layers);
                    }
                }
            }

            let Some(current_index) = index_of_layer(layered_graph.layers(), &layer) else {
                continue;
            };

            let mut after_layers: Vec<LayerRef> = Vec::new();
            if out_max > 0 {
                for offset in 0..out_max as usize {
                    let new_layer = Layer::new(&graph_ref);
                    layered_graph
                        .layers_mut()
                        .insert(current_index + 1 + offset, new_layer.clone());
                    after_layers.push(new_layer);
                }
            }
            if !after_layers.is_empty() {
                for (_, info) in &high_degree_nodes {
                    for root in &info.out_tree_roots {
                        move_tree(root, EdgeSelector::Outgoing, &after_layers);
                    }
                }
            }
        }

        layered_graph.layers_mut().retain(|layer| {
            !layer.lock().nodes().is_empty()
        });

        progress_monitor.done();
    }
}

#[derive(Clone)]
struct HighDegreeNodeInformation {
    inc_trees_max_height: i32,
    inc_tree_roots: Vec<LNodeRef>,
    out_trees_max_height: i32,
    out_tree_roots: Vec<LNodeRef>,
}

impl Default for HighDegreeNodeInformation {
    fn default() -> Self {
        Self {
            inc_trees_max_height: -1,
            inc_tree_roots: Vec::new(),
            out_trees_max_height: -1,
            out_tree_roots: Vec::new(),
        }
    }
}

#[derive(Clone, Copy)]
enum EdgeSelector {
    Incoming,
    Outgoing,
    Connected,
}

impl HighDegreeNodeLayeringProcessor {
    fn collect_high_degree_nodes(
        &self,
        layer: &LayerRef,
    ) -> (Vec<(LNodeRef, HighDegreeNodeInformation)>, i32, i32) {
        let nodes = layer
            .lock().nodes().clone();

        let mut high_degree_nodes = Vec::new();
        let mut inc_max = -1;
        let mut out_max = -1;

        for node in nodes {
            if !self.is_high_degree_node(&node) {
                continue;
            }
            let info = self.calculate_information(&node);
            inc_max = inc_max.max(info.inc_trees_max_height);
            out_max = out_max.max(info.out_trees_max_height);
            high_degree_nodes.push((node, info));
        }

        (high_degree_nodes, inc_max, out_max)
    }

    fn is_high_degree_node(&self, node: &LNodeRef) -> bool {
        self.degree(node) >= self.degree_threshold
    }

    fn degree(&self, node: &LNodeRef) -> i32 {
        selected_edges(node, EdgeSelector::Connected).len() as i32
    }

    fn calculate_information(&self, high_degree_node: &LNodeRef) -> HighDegreeNodeInformation {
        let mut info = HighDegreeNodeInformation::default();

        for incoming_edge in selected_edges(high_degree_node, EdgeSelector::Incoming) {
            let is_self_loop = incoming_edge
                .lock().is_self_loop();
            if is_self_loop {
                continue;
            }

            let Some(source) = source_node(&incoming_edge) else {
                continue;
            };

            if self.has_single_connection(&source, EdgeSelector::Outgoing) {
                let tree_height =
                    self.is_tree_root(&source, EdgeSelector::Outgoing, EdgeSelector::Incoming);
                if tree_height == -1 {
                    continue;
                }
                info.inc_trees_max_height = info.inc_trees_max_height.max(tree_height);
                info.inc_tree_roots.push(source);
            }
        }

        for outgoing_edge in selected_edges(high_degree_node, EdgeSelector::Outgoing) {
            let is_self_loop = outgoing_edge
                .lock().is_self_loop();
            if is_self_loop {
                continue;
            }

            let Some(target) = target_node(&outgoing_edge) else {
                continue;
            };

            if self.has_single_connection(&target, EdgeSelector::Incoming) {
                let tree_height =
                    self.is_tree_root(&target, EdgeSelector::Incoming, EdgeSelector::Outgoing);
                if tree_height == -1 {
                    continue;
                }
                info.out_trees_max_height = info.out_trees_max_height.max(tree_height);
                info.out_tree_roots.push(target);
            }
        }

        info
    }

    fn has_single_connection(&self, node: &LNodeRef, selector: EdgeSelector) -> bool {
        let mut connection: Option<LNodeRef> = None;

        for edge in selected_edges(node, selector) {
            let Some(other_node) = other_node(&edge, node) else {
                continue;
            };

            if let Some(existing) = &connection {
                if !Arc::ptr_eq(existing, &other_node) {
                    return false;
                }
            } else {
                connection = Some(other_node);
            }
        }

        true
    }

    fn is_tree_root(
        &self,
        root: &LNodeRef,
        ancestor_edges: EdgeSelector,
        descendant_edges: EdgeSelector,
    ) -> i32 {
        if self.is_high_degree_node(root) {
            return -1;
        }

        if !self.has_single_connection(root, ancestor_edges) {
            return -1;
        }

        let descendant_edges_list = selected_edges(root, descendant_edges);
        if descendant_edges_list.is_empty() {
            return 1;
        }

        let mut current_height = 0;
        for edge in descendant_edges_list {
            let Some(other) = other_node(&edge, root) else {
                return -1;
            };
            let subtree_height = self.is_tree_root(&other, ancestor_edges, descendant_edges);
            if subtree_height == -1 {
                return -1;
            }

            current_height = current_height.max(subtree_height);
            if current_height > self.tree_height_threshold - 1 {
                return -1;
            }
        }

        current_height + 1
    }

}

fn move_tree(root: &LNodeRef, edges: EdgeSelector, layers: &[LayerRef]) {
    if layers.is_empty() {
        return;
    }

    LNode::set_layer(root, Some(layers[0].clone()));
    if layers.len() == 1 {
        return;
    }

    for edge in selected_edges(root, edges) {
        let Some(other) = other_node(&edge, root) else {
            continue;
        };
        move_tree(&other, edges, &layers[1..]);
    }
}

fn selected_edges(node: &LNodeRef, selector: EdgeSelector) -> Vec<LEdgeRef> {
    let node_guard = node.lock();

    match selector {
        EdgeSelector::Incoming => node_guard.incoming_edges(),
        EdgeSelector::Outgoing => node_guard.outgoing_edges(),
        EdgeSelector::Connected => node_guard.connected_edges(),
    }
}

fn source_node(edge: &LEdgeRef) -> Option<LNodeRef> {
    edge.lock().source()
        .and_then(|port| port.lock().node())
}

fn target_node(edge: &LEdgeRef) -> Option<LNodeRef> {
    edge.lock().target()
        .and_then(|port| port.lock().node())
}

fn other_node(edge: &LEdgeRef, node: &LNodeRef) -> Option<LNodeRef> {
    let source = source_node(edge)?;
    let target = target_node(edge)?;
    if Arc::ptr_eq(&source, node) {
        return Some(target);
    }
    if Arc::ptr_eq(&target, node) {
        return Some(source);
    }
    None
}

fn graph_ref_for(layered_graph: &LGraph) -> LGraphRef {
    if let Some(layer) = layered_graph.layers().first() {
        if let Some(graph_ref) = layer
            .lock().graph()
        {
            return graph_ref;
        }
    }
    if let Some(node) = layered_graph.layerless_nodes().first() {
        if let Some(graph_ref) = node.lock().graph() {
            return graph_ref;
        }
    }
    LGraph::new()
}

fn index_of_layer(layers: &[LayerRef], target: &LayerRef) -> Option<usize> {
    layers.iter().position(|layer| Arc::ptr_eq(layer, target))
}
