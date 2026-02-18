use std::collections::HashMap;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkPadding, KVector, KVectorChain};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkEdgeRef, ElkEdgeSection, ElkGraphElementRef, ElkNodeRef,
};

use crate::org::eclipse::elk::alg::mrtree::graph::{TEdge, TGraph, TGraphRef, TNode};
use crate::org::eclipse::elk::alg::mrtree::i_graph_importer::IGraphImporter;
use crate::org::eclipse::elk::alg::mrtree::options::internal_properties::{Origin, OriginId};
use crate::org::eclipse::elk::alg::mrtree::options::InternalProperties;
use crate::org::eclipse::elk::alg::mrtree::options::MrTreeOptions;

pub struct ElkGraphImporter {
    node_map: HashMap<OriginId, ElkNodeRef>,
    edge_map: HashMap<OriginId, ElkEdgeRef>,
}

impl ElkGraphImporter {
    pub fn new() -> Self {
        ElkGraphImporter {
            node_map: HashMap::new(),
            edge_map: HashMap::new(),
        }
    }

    fn origin_id(element: &ElkGraphElementRef) -> OriginId {
        match element {
            ElkGraphElementRef::Node(node) => Rc::as_ptr(node) as usize,
            ElkGraphElementRef::Edge(edge) => Rc::as_ptr(edge) as usize,
            ElkGraphElementRef::Port(port) => Rc::as_ptr(port) as usize,
            ElkGraphElementRef::Label(label) => Rc::as_ptr(label) as usize,
        }
    }
}

impl Default for ElkGraphImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphImporter<ElkNodeRef> for ElkGraphImporter {
    fn import_graph(&mut self, elkgraph: &ElkNodeRef) -> Option<TGraphRef> {
        let t_graph = TGraph::new();

        let graph_props = {
            let mut root = elkgraph.borrow_mut();
            root.connectable()
                .shape()
                .graph_element()
                .properties()
                .clone()
        };
        if let Ok(mut graph_guard) = t_graph.lock() {
            graph_guard.properties_mut().copy_properties(&graph_props);
        }

        let origin_id = Self::origin_id(&ElkGraphElementRef::Node(elkgraph.clone()));
        if let Ok(mut graph_guard) = t_graph.lock() {
            graph_guard.set_property(InternalProperties::ORIGIN, Some(Origin::ElkNode(origin_id)));
        }
        self.node_map.insert(origin_id, elkgraph.clone());

        let children: Vec<ElkNodeRef> = {
            let mut root = elkgraph.borrow_mut();
            root.children().iter().cloned().collect()
        };
        let mut elem_map: HashMap<
            OriginId,
            crate::org::eclipse::elk::alg::mrtree::graph::TNodeRef,
        > = HashMap::new();

        for (index, elknode) in children.iter().enumerate() {
            let label = {
                let mut node_mut = elknode.borrow_mut();
                let labels: Vec<_> = node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect();
                labels
                    .first()
                    .map(|label| label.borrow().text().to_string())
                    .unwrap_or_default()
            };

            let t_node = TNode::new_with_label(index as i32, Some(t_graph.clone()), label);

            let node_props = {
                let mut node_mut = elknode.borrow_mut();
                node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties()
                    .clone()
            };

            if let Ok(mut node_guard) = t_node.lock() {
                node_guard.properties_mut().copy_properties(&node_props);
                let (x, y, w, h) = {
                    let mut node_mut = elknode.borrow_mut();
                    let shape = node_mut.connectable().shape();
                    (shape.x(), shape.y(), shape.width(), shape.height())
                };
                {
                    let pos = node_guard.position();
                    pos.x = x + w / 2.0;
                    pos.y = y + h / 2.0;
                }
                {
                    let size = node_guard.size();
                    size.x = w.max(1.0);
                    size.y = h.max(1.0);
                }
            }

            let node_origin = Self::origin_id(&ElkGraphElementRef::Node(elknode.clone()));
            if let Ok(mut node_guard) = t_node.lock() {
                node_guard.set_property(
                    InternalProperties::ORIGIN,
                    Some(Origin::ElkNode(node_origin)),
                );
            }
            self.node_map.insert(node_origin, elknode.clone());
            elem_map.insert(node_origin, t_node);
        }

        let mut seen_edges: HashMap<OriginId, ()> = HashMap::new();
        for elknode in &children {
            for elkedge in ElkGraphUtil::all_outgoing_edges(elknode) {
                let edge_guard = elkedge.borrow();
                if edge_guard.is_hierarchical()
                    || edge_guard.is_selfloop()
                    || edge_guard.is_hyperedge()
                {
                    continue;
                }
                let origin_id = Self::origin_id(&ElkGraphElementRef::Edge(elkedge.clone()));
                if seen_edges.contains_key(&origin_id) {
                    continue;
                }
                drop(edge_guard);

                let source_node = Self::origin_id(&ElkGraphElementRef::Node(elknode.clone()));
                let target_node = {
                    let edge_borrow = elkedge.borrow();
                    let target_shape = edge_borrow.targets_ro().get(0);
                    target_shape.and_then(|shape| ElkGraphUtil::connectable_shape_to_node(&shape))
                };

                let Some(target_node) = target_node else {
                    continue;
                };
                let target_origin = Self::origin_id(&ElkGraphElementRef::Node(target_node.clone()));

                let source = elem_map.get(&source_node).cloned();
                let target = elem_map.get(&target_origin).cloned();

                if let (Some(source), Some(target)) = (source, target) {
                    let t_edge = TEdge::new(&source, &target);
                    let edge_props = {
                        let mut edge_mut = elkedge.borrow_mut();
                        edge_mut.element().properties().clone()
                    };
                    if let Ok(mut edge_guard) = t_edge.lock() {
                        edge_guard
                            .element_mut()
                            .properties_mut()
                            .copy_properties(&edge_props);
                        edge_guard.set_property(
                            InternalProperties::ORIGIN,
                            Some(Origin::ElkEdge(origin_id)),
                        );
                    }

                    if let Ok(mut graph_guard) = t_graph.lock() {
                        graph_guard.edges_mut().push(t_edge);
                    }

                    self.edge_map.insert(origin_id, elkedge.clone());
                    seen_edges.insert(origin_id, ());
                }
            }
        }

        Some(t_graph)
    }

    fn apply_layout(&self, tgraph: &TGraphRef) {
        let origin = tgraph
            .lock()
            .ok()
            .and_then(|mut g| g.get_property(InternalProperties::ORIGIN));
        let Some(Origin::ElkNode(root_id)) = origin else {
            return;
        };
        let Some(elkgraph) = self.node_map.get(&root_id) else {
            return;
        };

        let nodes = tgraph
            .lock()
            .ok()
            .map(|g| g.nodes().clone())
            .unwrap_or_default();
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for node in &nodes {
            if let Ok(node_guard) = node.lock() {
                let pos = node_guard.position_ref();
                let size = node_guard.size_ref();
                min_x = min_x.min(pos.x - size.x / 2.0);
                min_y = min_y.min(pos.y - size.y / 2.0);
                max_x = max_x.max(pos.x + size.x / 2.0);
                max_y = max_y.max(pos.y + size.y / 2.0);
            }
        }

        let padding = {
            let mut root_mut = elkgraph.borrow_mut();
            let props = root_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            props
                .get_property(MrTreeOptions::PADDING)
                .unwrap_or_else(ElkPadding::new)
        };

        for node in &nodes {
            let origin = node
                .lock()
                .ok()
                .and_then(|mut guard| guard.get_property(InternalProperties::ORIGIN));
            let Some(Origin::ElkNode(node_id)) = origin else {
                continue;
            };
            let Some(elk_node) = self.node_map.get(&node_id) else {
                continue;
            };
            if let Ok(node_guard) = node.lock() {
                let pos = node_guard.position_ref();
                let props = node_guard.properties().clone();
                let mut elk_node_mut = elk_node.borrow_mut();
                elk_node_mut
                    .connectable()
                    .shape()
                    .set_location(pos.x, pos.y);
                elk_node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .copy_properties(&props);
            }
        }

        let edges = tgraph
            .lock()
            .ok()
            .map(|g| g.edges().clone())
            .unwrap_or_default();
        for edge in edges {
            let origin = edge
                .lock()
                .ok()
                .and_then(|mut guard| guard.get_property(InternalProperties::ORIGIN));
            let Some(Origin::ElkEdge(edge_id)) = origin else {
                continue;
            };
            let Some(elk_edge) = self.edge_map.get(&edge_id) else {
                continue;
            };
            let mut bend_points = edge
                .lock()
                .ok()
                .map(|guard| guard.bend_points_ref().clone());
            let Some(mut bend_points_value) = bend_points.take() else {
                continue;
            };

            if bend_points_value.size() < 2 {
                let endpoints = edge.lock().ok().and_then(|guard| {
                    let source = guard.source()?;
                    let target = guard.target()?;
                    let source_center = source.lock().ok().map(|source_guard| {
                        let pos = source_guard.position_ref();
                        let size = source_guard.size_ref();
                        KVector::with_values(pos.x + size.x / 2.0, pos.y + size.y / 2.0)
                    })?;
                    let target_center = target.lock().ok().map(|target_guard| {
                        let pos = target_guard.position_ref();
                        let size = target_guard.size_ref();
                        KVector::with_values(pos.x + size.x / 2.0, pos.y + size.y / 2.0)
                    })?;
                    Some((source_center, target_center))
                });
                let Some((source_center, target_center)) = endpoints else {
                    continue;
                };
                bend_points_value = KVectorChain::new();
                bend_points_value.add_vector(source_center);
                bend_points_value.add_vector(target_center);
            }

            let section = {
                let mut edge_mut = elk_edge.borrow_mut();
                if let Some(section) = edge_mut.sections().get(0) {
                    section
                } else {
                    let section = ElkEdgeSection::new();
                    edge_mut.sections().add(section.clone());
                    section
                }
            };
            ElkUtil::apply_vector_chain(&bend_points_value, &section);
        }

        let padding_horizontal = padding.left + padding.right;
        let padding_vertical = padding.top + padding.bottom;
        let width = max_x - min_x + padding_horizontal;
        let height = max_y - min_y + padding_vertical;
        let fixed_size = {
            let mut root_mut = elkgraph.borrow_mut();
            let props = root_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            props
                .get_property(CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE)
                .unwrap_or(false)
        };
        if !fixed_size {
            ElkUtil::resize_node_with(elkgraph, width, height, false, false);
        }

        let mut root_mut = elkgraph.borrow_mut();
        root_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(
                CoreOptions::CHILD_AREA_WIDTH,
                Some(width - padding_horizontal),
            );
        root_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(
                CoreOptions::CHILD_AREA_HEIGHT,
                Some(height - padding_vertical),
            );
    }
}
