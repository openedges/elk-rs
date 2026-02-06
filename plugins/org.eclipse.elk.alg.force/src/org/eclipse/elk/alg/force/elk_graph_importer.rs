use std::collections::HashMap;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_padding::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkBendPoint, ElkEdgeRef, ElkEdgeSection, ElkGraphElementRef, ElkGraphFactory, ElkLabelRef,
    ElkNodeRef,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

use crate::org::eclipse::elk::alg::force::graph::{FEdge, FGraph, FLabel, FNode};
use crate::org::eclipse::elk::alg::force::i_graph_importer::IGraphImporter;
use crate::org::eclipse::elk::alg::force::options::{ForceOptions, InternalProperties, Origin, OriginId};
use org_eclipse_elk_core::org::eclipse::elk::core::UnsupportedGraphException;

pub struct ElkGraphImporter {
    node_map: HashMap<OriginId, ElkNodeRef>,
    edge_map: HashMap<OriginId, ElkEdgeRef>,
    label_map: HashMap<OriginId, ElkLabelRef>,
}

impl ElkGraphImporter {
    pub fn new() -> Self {
        ElkGraphImporter {
            node_map: HashMap::new(),
            edge_map: HashMap::new(),
            label_map: HashMap::new(),
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

    fn ensure_single_section(edge: &ElkEdgeRef) -> Option<org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeSectionRef> {
        let mut edge_mut = edge.borrow_mut();
        let sections = edge_mut.sections();
        if sections.is_empty() {
            let section = ElkGraphFactory::instance().create_elk_edge_section();
            sections.add(section.clone());
            return Some(section);
        }
        if sections.len() > 1 {
            sections.retain_last();
        }
        let section = sections.get(0)?;
        {
            let mut section_mut = section.borrow_mut();
            section_mut.bend_points().clear();
            section_mut.set_start_x(0.0);
            section_mut.set_start_y(0.0);
            section_mut.set_end_x(0.0);
            section_mut.set_end_y(0.0);
        }
        Some(section)
    }

    fn create_bend_point(section: &mut ElkEdgeSection, x: f64, y: f64) {
        let bend = ElkBendPoint::new();
        {
            let mut bend_mut = bend.borrow_mut();
            bend_mut.set_x(x);
            bend_mut.set_y(y);
        }
        section.bend_points().push(bend);
    }
}

impl Default for ElkGraphImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphImporter<ElkNodeRef> for ElkGraphImporter {
    fn import_graph(&mut self, elkgraph: &ElkNodeRef) -> Option<FGraph> {
        let mut fgraph = FGraph::new();

        let graph_props = {
            let mut root = elkgraph.borrow_mut();
            root.connectable()
                .shape()
                .graph_element()
                .properties()
                .clone()
        };
        fgraph.copy_properties(&graph_props);

        let origin_id = Self::origin_id(&ElkGraphElementRef::Node(elkgraph.clone()));
        fgraph.set_property(InternalProperties::ORIGIN, Some(Origin::ElkNode(origin_id)));
        self.node_map.insert(origin_id, elkgraph.clone());

        let children: Vec<ElkNodeRef> = {
            let mut root = elkgraph.borrow_mut();
            root.children().iter().cloned().collect()
        };
        let mut elem_map: HashMap<OriginId, crate::org::eclipse::elk::alg::force::graph::FNodeRef> =
            HashMap::new();

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

            let f_node = FNode::new_with_label(label);
            {
                let mut node_guard = f_node.lock().ok()?;
                node_guard.set_id(index);

                let node_props = {
                    let mut node_mut = elknode.borrow_mut();
                    node_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .properties()
                        .clone()
                };
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

                let node_origin = Self::origin_id(&ElkGraphElementRef::Node(elknode.clone()));
                node_guard.set_property(InternalProperties::ORIGIN, Some(Origin::ElkNode(node_origin)));
                self.node_map.insert(node_origin, elknode.clone());
                elem_map.insert(node_origin, f_node.clone());
            }

            fgraph.nodes_mut().push(f_node.clone());

            let port_constraints = {
                let mut node_mut = elknode.borrow_mut();
                let props = node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut();
                if props.has_property(ForceOptions::PORT_CONSTRAINTS) {
                    props
                        .get_property(ForceOptions::PORT_CONSTRAINTS)
                        .unwrap_or(PortConstraints::Undefined)
                } else {
                    PortConstraints::Undefined
                }
            };
            if port_constraints == PortConstraints::Undefined {
                let mut node_mut = elknode.borrow_mut();
                node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .set_property(ForceOptions::PORT_CONSTRAINTS, Some(PortConstraints::Free));
            }
        }

        let mut seen_edges: HashMap<OriginId, ()> = HashMap::new();
        for elknode in &children {
            for elkedge in ElkGraphUtil::all_outgoing_edges(elknode) {
                let edge_guard = elkedge.borrow();
                if edge_guard.is_hyperedge() {
                    panic!(
                        "{}",
                        UnsupportedGraphException::new("Graph must not contain hyperedges.")
                    );
                }
                if edge_guard.is_hierarchical() || edge_guard.is_selfloop() {
                    continue;
                }
                drop(edge_guard);

                let origin_id = Self::origin_id(&ElkGraphElementRef::Edge(elkedge.clone()));
                if seen_edges.contains_key(&origin_id) {
                    continue;
                }

                let source_node_id = Self::origin_id(&ElkGraphElementRef::Node(elknode.clone()));
                let target_node = {
                    let edge_borrow = elkedge.borrow();
                    let target_shape = edge_borrow.targets_ro().get(0);
                    target_shape.and_then(|shape| ElkGraphUtil::connectable_shape_to_node(&shape))
                };
                let Some(target_node) = target_node else { continue };
                let target_node_id = Self::origin_id(&ElkGraphElementRef::Node(target_node.clone()));

                let source = elem_map.get(&source_node_id).cloned();
                let target = elem_map.get(&target_node_id).cloned();

                if let (Some(source), Some(target)) = (source, target) {
                    let f_edge = FEdge::new();
                    {
                        let mut edge_guard = f_edge.lock().ok()?;
                        let edge_props = {
                            let mut edge_mut = elkedge.borrow_mut();
                            edge_mut.element().properties().clone()
                        };
                        edge_guard.properties_mut().copy_properties(&edge_props);
                        edge_guard.set_property(InternalProperties::ORIGIN, Some(Origin::ElkEdge(origin_id)));
                    }
                    FEdge::set_source(&f_edge, Some(source));
                    FEdge::set_target(&f_edge, Some(target));

                    fgraph.edges_mut().push(f_edge.clone());
                    self.edge_map.insert(origin_id, elkedge.clone());
                    seen_edges.insert(origin_id, ());

                    let labels: Vec<ElkLabelRef> = {
                        let mut edge_mut = elkedge.borrow_mut();
                        edge_mut.element().labels().iter().cloned().collect()
                    };
                    for label in labels {
                        let label_text = label.borrow().text().to_string();
                        let f_label = FLabel::new(&f_edge, label_text);
                        {
                            let mut label_guard = f_label.lock().ok()?;
                            let label_props = {
                                let mut label_mut = label.borrow_mut();
                                label_mut.shape().graph_element().properties().clone()
                            };
                            label_guard.properties_mut().copy_properties(&label_props);
                            let origin = Self::origin_id(&ElkGraphElementRef::Label(label.clone()));
                            label_guard.set_property(InternalProperties::ORIGIN, Some(Origin::ElkLabel(origin)));
                            self.label_map.insert(origin, label.clone());

                            let (w, h) = {
                                let mut label_mut = label.borrow_mut();
                                let shape = label_mut.shape();
                                (shape.width(), shape.height())
                            };
                            label_guard.size().x = w.max(1.0);
                            label_guard.size().y = h.max(1.0);
                            label_guard.refresh_position();
                        }
                        fgraph.labels_mut().push(f_label);
                    }
                }
            }
        }

        Some(fgraph)
    }

    fn apply_layout(&self, fgraph: &FGraph) {
        let origin = {
            let mut props = fgraph.properties().clone();
            props.get_property(InternalProperties::ORIGIN)
        };
        let Some(Origin::ElkNode(root_id)) = origin else { return };
        let Some(elkgraph) = self.node_map.get(&root_id) else { return };

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for node in fgraph.nodes() {
            if let Ok(node_guard) = node.lock() {
                let pos = node_guard.position_ref();
                let size = node_guard.size_ref();
                min_x = min_x.min(pos.x - size.x / 2.0);
                min_y = min_y.min(pos.y - size.y / 2.0);
                max_x = max_x.max(pos.x + size.x / 2.0);
                max_y = max_y.max(pos.y + size.y / 2.0);
            }
        }
        for bend in fgraph.bendpoints() {
            if let Ok(bend_guard) = bend.lock() {
                let pos = bend_guard.position_ref();
                let size = bend_guard.size_ref();
                min_x = min_x.min(pos.x - size.x / 2.0);
                min_y = min_y.min(pos.y - size.y / 2.0);
                max_x = max_x.max(pos.x + size.x / 2.0);
                max_y = max_y.max(pos.y + size.y / 2.0);
            }
        }

        let padding = {
            let mut root_mut = elkgraph.borrow_mut();
            root_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(ForceOptions::PADDING)
                .unwrap_or_else(|| ElkPadding::with_any(0.0))
        };
        let offset = KVector::with_values(padding.left - min_x, padding.top - min_y);

        for node in fgraph.nodes() {
            let origin = node
                .lock()
                .ok()
                .and_then(|mut node_guard| node_guard.get_property(InternalProperties::ORIGIN));
            let Some(Origin::ElkNode(node_id)) = origin else { continue };
            let Some(elknode) = self.node_map.get(&node_id) else { continue };

            if let Ok(node_guard) = node.lock() {
                let mut node_pos = KVector::from_vector(node_guard.position_ref());
                node_pos.add(&offset);
                let mut elk_mut = elknode.borrow_mut();
                let (width, height) = {
                    let shape = elk_mut.connectable().shape();
                    (shape.width(), shape.height())
                };
                elk_mut
                    .connectable()
                    .shape()
                    .set_location(node_pos.x - width / 2.0, node_pos.y - height / 2.0);
            }
        }

        for edge in fgraph.edges() {
            let origin = edge
                .lock()
                .ok()
                .and_then(|mut edge_guard| edge_guard.get_property(InternalProperties::ORIGIN));
            let Some(Origin::ElkEdge(edge_id)) = origin else { continue };
            let Some(elkedge) = self.edge_map.get(&edge_id) else { continue };

            let Some(section) = Self::ensure_single_section(elkedge) else { continue };
            let mut section_mut = section.borrow_mut();

            if let Ok(edge_guard) = edge.lock() {
                if let Some(mut start_location) = edge_guard.source_point() {
                    start_location.add(&offset);
                    section_mut.set_start_x(start_location.x);
                    section_mut.set_start_y(start_location.y);
                }

                for bend in edge_guard.bendpoints() {
                    if let Ok(bend_guard) = bend.lock() {
                        let mut position = KVector::from_vector(bend_guard.position_ref());
                        position.add(&offset);
                        Self::create_bend_point(&mut section_mut, position.x, position.y);
                    }
                }

                if let Some(mut end_location) = edge_guard.target_point() {
                    end_location.add(&offset);
                    section_mut.set_end_x(end_location.x);
                    section_mut.set_end_y(end_location.y);
                }
            }
        }

        for label in fgraph.labels() {
            let origin = label
                .lock()
                .ok()
                .and_then(|mut label_guard| label_guard.get_property(InternalProperties::ORIGIN));
            let Some(Origin::ElkLabel(label_id)) = origin else { continue };
            let Some(elklabel) = self.label_map.get(&label_id) else { continue };

            if let Ok(label_guard) = label.lock() {
                let mut label_pos = KVector::from_vector(label_guard.position_ref());
                label_pos.add(&offset);
                elklabel
                    .borrow_mut()
                    .shape()
                    .set_location(label_pos.x, label_pos.y);
            }
        }

        let horizontal = padding.left + padding.right;
        let vertical = padding.top + padding.bottom;
        let width = (max_x - min_x) + horizontal;
        let height = (max_y - min_y) + vertical;
        let fixed_graph_size = {
            let mut root_mut = elkgraph.borrow_mut();
            root_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE)
                .unwrap_or(false)
        };
        if !fixed_graph_size {
            ElkUtil::resize_node_with(elkgraph, width, height, false, true);
        }
        {
            let mut root_mut = elkgraph.borrow_mut();
            root_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .set_property(CoreOptions::CHILD_AREA_WIDTH, Some(width - horizontal));
            root_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .set_property(CoreOptions::CHILD_AREA_HEIGHT, Some(height - vertical));
        }
    }
}
