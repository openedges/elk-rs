use std::collections::HashMap;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::nodespacing::node_dimension_calculation::NodeDimensionCalculation;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::internal_properties::InternalProperties;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::node::Node;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMath, ElkPadding, ElkRectangle, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::ElkGraphAdapters;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, Random};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkEdgeRef, ElkEdgeSectionRef, ElkGraphFactory, ElkNodeRef};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

use crate::org::eclipse::elk::alg::spore::graph::Graph;
use crate::org::eclipse::elk::alg::spore::i_graph_importer::IGraphImporter;
use crate::org::eclipse::elk::alg::spore::options::{RootSelection, SporeCompactionOptions};

pub struct ElkGraphImporter {
    node_map: HashMap<KVector, ElkNodeRef>,
    elk_graph: Option<ElkNodeRef>,
    spacing_node_node: f64,
}

impl ElkGraphImporter {
    pub fn new() -> Self {
        ElkGraphImporter {
            node_map: HashMap::new(),
            elk_graph: None,
            spacing_node_node: 0.0,
        }
    }
}

impl Default for ElkGraphImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphImporter<ElkNodeRef> for ElkGraphImporter {
    fn import_graph(&mut self, input_graph: &ElkNodeRef) -> Graph {
        self.elk_graph = Some(input_graph.clone());
        self.node_map.clear();

        let adapter = ElkGraphAdapters::adapt(input_graph.clone());
        NodeDimensionCalculation::calculate_node_margins(&adapter);

        let preferred_root_id = property(input_graph, SporeCompactionOptions::PROCESSING_ORDER_PREFERRED_ROOT);
        let cost_function_id =
            property(input_graph, SporeCompactionOptions::PROCESSING_ORDER_SPANNING_TREE_COST_FUNCTION)
                .unwrap_or_default();
        let tree_construction = property(input_graph, SporeCompactionOptions::PROCESSING_ORDER_TREE_CONSTRUCTION)
            .unwrap_or_default();
        let compaction_strategy =
            property(input_graph, SporeCompactionOptions::COMPACTION_COMPACTION_STRATEGY).unwrap_or_default();
        let root_selection = property(input_graph, SporeCompactionOptions::PROCESSING_ORDER_ROOT_SELECTION)
            .unwrap_or_default();
        self.spacing_node_node = property(input_graph, SporeCompactionOptions::SPACING_NODE_NODE).unwrap_or(0.0);

        let mut graph = Graph::new(cost_function_id, tree_construction, compaction_strategy);
        let debug_mode = property(input_graph, SporeCompactionOptions::DEBUG_MODE).unwrap_or(false);
        graph.set_property(InternalProperties::DEBUG_SVG, Some(debug_mode));
        graph.orthogonal_compaction = property(input_graph, SporeCompactionOptions::COMPACTION_ORTHOGONAL)
            .unwrap_or(false);

        let children = {
            let mut graph_mut = input_graph.borrow_mut();
            graph_mut.children().iter().cloned().collect::<Vec<_>>()
        };
        if children.is_empty() {
            return graph;
        }

        let mut random = Random::new(0);
        for elk_node in children {
            let (x, y, width, height) = {
                let mut node_mut = elk_node.borrow_mut();
                let shape = node_mut.connectable().shape();
                (shape.x(), shape.y(), shape.width(), shape.height())
            };

            let half_width = width / 2.0;
            let half_height = height / 2.0;
            let mut vertex = KVector::with_values(x + half_width, y + half_height);

            while self.node_map.contains_key(&vertex) {
                vertex.wiggle(&mut random, 0.001);
            }

            let margin = property(&elk_node, CoreOptions::MARGINS).unwrap_or_default();
            let rect = ElkRectangle::with_values(
                vertex.x - half_width - self.spacing_node_node / 2.0 - margin.left,
                vertex.y - half_height - self.spacing_node_node / 2.0 - margin.top,
                width + self.spacing_node_node + margin.left + margin.right,
                height + self.spacing_node_node + margin.top + margin.bottom,
            );

            let node = Node::new(vertex, rect);
            graph.vertices.push(node);
            self.node_map.insert(vertex, elk_node.clone());
        }

        graph.rebuild_index_map();

        match root_selection {
            RootSelection::Fixed => {
                if preferred_root_id.is_none() {
                    graph.preferred_root_index = Some(0);
                } else if let Some(preferred_root_id) = preferred_root_id {
                    for (idx, node) in graph.vertices.iter().enumerate() {
                        if let Some(elk_node) = self.node_map.get(&node.original_vertex) {
                            let id = elk_node
                                .borrow_mut()
                                .connectable()
                                .shape()
                                .graph_element()
                                .identifier()
                                .map(|value| value.to_string());
                            if let Some(id) = id {
                                if id == preferred_root_id {
                                    graph.preferred_root_index = Some(idx);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            RootSelection::CenterNode => {
                let (x, y, width, height) = {
                    let mut graph_mut = input_graph.borrow_mut();
                    let shape = graph_mut.connectable().shape();
                    (shape.x(), shape.y(), shape.width(), shape.height())
                };
                let mut center = KVector::with_values(width, height);
                center.scale(0.5);
                center.add_values(x, y);

                let mut closest = f64::INFINITY;
                let mut best_idx = None;
                for (idx, node) in graph.vertices.iter().enumerate() {
                    let distance = node.original_vertex.distance(&center);
                    if distance < closest {
                        closest = distance;
                        best_idx = Some(idx);
                    }
                }
                graph.preferred_root_index = best_idx;
            }
        }

        graph
    }

    fn update_graph(&mut self, graph: &mut Graph) {
        let mut updated_map = HashMap::new();
        graph.t_edges = None;
        graph.tree = None;

        for node in &mut graph.vertices {
            let Some(elk_node) = self.node_map.get(&node.original_vertex).cloned() else {
                continue;
            };
            node.original_vertex = node.rect.get_center();
            updated_map.insert(node.original_vertex, elk_node);
        }

        self.node_map = updated_map;
        graph.rebuild_index_map();
    }

    fn apply_positions(&mut self, graph: &Graph) {
        let Some(elk_graph) = &self.elk_graph else {
            return;
        };

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for node in &graph.vertices {
            let Some(elk_node) = self.node_map.get(&node.original_vertex) else {
                continue;
            };
            let (x, y, width, height) = {
                let mut node_mut = elk_node.borrow_mut();
                let shape = node_mut.connectable().shape();
                shape.set_location(node.rect.x, node.rect.y);
                (shape.x(), shape.y(), shape.width(), shape.height())
            };
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + width);
            max_y = max_y.max(y + height);
        }

        if min_x == f64::INFINITY || min_y == f64::INFINITY {
            return;
        }

        let padding = property(elk_graph, SporeCompactionOptions::PADDING).unwrap_or(ElkPadding::new());
        ElkUtil::resize_node_with(
            elk_graph,
            max_x - min_x + padding.left + padding.right,
            max_y - min_y + padding.top + padding.bottom,
            true,
            true,
        );
        ElkUtil::translate((elk_graph, -min_x + padding.left, -min_y + padding.top));

        let edges = {
            let mut graph_mut = elk_graph.borrow_mut();
            graph_mut.contained_edges().iter().cloned().collect::<Vec<_>>()
        };

        for edge in edges {
            let section = first_edge_section(&edge, true, true);
            let Some(section) = section else {
                continue;
            };

            let (source, target) = edge_endpoints(&edge);
            let (Some(source), Some(target)) = (source, target) else {
                continue;
            };

            let (sx, sy, sw, sh) = node_bounds(&source);
            let (tx, ty, tw, th) = node_bounds(&target);

            let mut start_location = KVector::with_values(sx + sw / 2.0, sy + sh / 2.0);
            let mut end_location = KVector::with_values(tx + tw / 2.0, ty + th / 2.0);

            let mut uv = end_location;
            uv.sub(&start_location);
            ElkMath::clip_vector(&mut uv, sw, sh);
            start_location.add(&uv);

            let mut vu = start_location;
            vu.sub(&end_location);
            ElkMath::clip_vector(&mut vu, tw, th);
            end_location.add(&vu);

            let mut section_mut = section.borrow_mut();
            section_mut.set_start_x(start_location.x);
            section_mut.set_start_y(start_location.y);
            section_mut.set_end_x(end_location.x);
            section_mut.set_end_y(end_location.y);
        }
    }
}

fn property<T: Clone + Send + Sync + 'static>(
    graph: &ElkNodeRef,
    property: &'static Property<T>,
) -> Option<T> {
    let mut graph_mut = graph.borrow_mut();
    graph_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(property)
}

fn node_bounds(node: &ElkNodeRef) -> (f64, f64, f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}

fn edge_endpoints(edge: &ElkEdgeRef) -> (Option<ElkNodeRef>, Option<ElkNodeRef>) {
    let (source, target) = {
        let edge_ref = edge.borrow();
        (edge_ref.sources_ro().get(0), edge_ref.targets_ro().get(0))
    };

    let source_node = source.and_then(|shape| ElkGraphUtil::connectable_shape_to_node(&shape));
    let target_node = target.and_then(|shape| ElkGraphUtil::connectable_shape_to_node(&shape));

    (source_node, target_node)
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
    if reset_section {
        if let Some(section) = &section {
            let mut section_mut = section.borrow_mut();
            section_mut.bend_points().clear();
            section_mut.set_start_x(0.0);
            section_mut.set_start_y(0.0);
            section_mut.set_end_x(0.0);
            section_mut.set_end_y(0.0);
        }
    }

    if remove_other_sections {
        sections.retain_last();
    }

    section
}
