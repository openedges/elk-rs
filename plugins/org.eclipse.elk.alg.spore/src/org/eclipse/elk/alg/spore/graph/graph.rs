use std::collections::{HashMap, HashSet};

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::Node;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::t_edge::TEdge;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::tree::Tree;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;

use crate::org::eclipse::elk::alg::spore::options::{CompactionStrategy, SpanningTreeCostFunction, TreeConstructionStrategy};

pub struct Graph {
    properties: MapPropertyHolder,
    pub vertices: Vec<Node>,
    pub t_edges: Option<HashSet<TEdge>>,
    pub tree_construction_strategy: TreeConstructionStrategy,
    pub tree: Option<Tree<Node>>,
    pub compaction_strategy: CompactionStrategy,
    pub cost_function: SpanningTreeCostFunction,
    pub preferred_root_index: Option<usize>,
    pub orthogonal_compaction: bool,
    node_index_map: HashMap<KVector, usize>,
}

impl Graph {
    pub fn new(
        cost_function: SpanningTreeCostFunction,
        tree_construction_strategy: TreeConstructionStrategy,
        compaction_strategy: CompactionStrategy,
    ) -> Self {
        Graph {
            properties: MapPropertyHolder::new(),
            vertices: Vec::new(),
            t_edges: None,
            tree_construction_strategy,
            tree: None,
            compaction_strategy,
            cost_function,
            preferred_root_index: None,
            orthogonal_compaction: false,
            node_index_map: HashMap::new(),
        }
    }

    pub fn rebuild_index_map(&mut self) {
        self.node_index_map.clear();
        for (idx, node) in self.vertices.iter().enumerate() {
            self.node_index_map.insert(node.original_vertex, idx);
        }
    }

    pub fn node_for_vertex(&self, vertex: &KVector) -> Option<&Node> {
        self.node_index_map
            .get(vertex)
            .and_then(|idx| self.vertices.get(*idx))
    }

    pub fn preferred_root(&self) -> Option<&Node> {
        self.preferred_root_index
            .and_then(|idx| self.vertices.get(idx))
    }

    pub fn sync_vertices_from_tree(&mut self) {
        let Some(tree) = &self.tree else {
            return;
        };

        let mut map: HashMap<KVector, Node> = HashMap::new();
        collect_tree_nodes(&mut map, tree);

        for node in &mut self.vertices {
            if let Some(updated) = map.get(&node.original_vertex) {
                node.vertex = updated.vertex;
                node.rect = updated.rect;
            }
        }
    }

    pub fn properties(&self) -> &MapPropertyHolder {
        &self.properties
    }

    pub fn properties_mut(&mut self) -> &mut MapPropertyHolder {
        &mut self.properties
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
    ) -> Option<T> {
        self.properties.get_property(property)
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
        value: Option<T>,
    ) {
        self.properties.set_property(property, value);
    }
}

fn collect_tree_nodes(map: &mut HashMap<KVector, Node>, tree: &Tree<Node>) {
    map.insert(tree.node.original_vertex, tree.node.clone());
    for child in &tree.children {
        collect_tree_nodes(map, child);
    }
}
