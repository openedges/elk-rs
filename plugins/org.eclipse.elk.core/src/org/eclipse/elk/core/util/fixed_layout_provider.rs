use std::rc::Rc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkEdgeRef, ElkGraphFactory, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

use crate::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use crate::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use crate::org::eclipse::elk::core::layout_arena_context::with_layout_arena;
use crate::org::eclipse::elk::core::math::{ElkPadding, KVector};
use crate::org::eclipse::elk::core::options::{
    CoreOptions, EdgeRouting, FixedLayouterOptions, SizeConstraint,
};
use crate::org::eclipse::elk::core::util::{ElkUtil, IElkProgressMonitor};

#[derive(Clone, Default)]
pub struct FixedLayoutProvider;

impl FixedLayoutProvider {
    pub fn new() -> Self {
        FixedLayoutProvider
    }
}

impl IGraphLayoutEngine for FixedLayoutProvider {
    fn layout(
        &mut self,
        layout_graph: &ElkNodeRef,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        progress_monitor.begin("Fixed Layout", 1.0);

        let edge_routing = with_layout_arena(|sync| {
            sync.node_id(layout_graph).and_then(|nid|
                sync.arena().node_properties[nid.idx()].get_property(CoreOptions::EDGE_ROUTING))
        }).flatten().unwrap_or_else(|| {
            with_node_properties_mut(layout_graph, |props|
                props.get_property(CoreOptions::EDGE_ROUTING).unwrap_or(EdgeRouting::Undefined))
        });

        let mut maxx: f64 = 0.0;
        let mut maxy: f64 = 0.0;

        let children: Vec<ElkNodeRef> = {
            let mut layout_mut = layout_graph.borrow_mut();
            layout_mut.children().iter().cloned().collect()
        };

        for node in children {
            if let Some(pos) = with_node_properties_mut(&node, |props| {
                props.get_property(FixedLayouterOptions::POSITION)
            }) {
                set_node_location(&node, &pos);

                let constraints = with_node_properties_mut(&node, |props| {
                    props
                        .get_property(FixedLayouterOptions::NODE_SIZE_CONSTRAINTS)
                        .unwrap_or_default()
                });
                if constraints.contains(&SizeConstraint::MinimumSize) {
                    if let Some(min_size) = with_node_properties_mut(&node, |props| {
                        props.get_property(FixedLayouterOptions::NODE_SIZE_MINIMUM)
                    }) {
                        if min_size.x > 0.0 && min_size.y > 0.0 {
                            ElkUtil::resize_node_with(&node, min_size.x, min_size.y, true, true);
                        }
                    }
                }
            }

            let (node_x, node_y, node_w, node_h) = node_bounds(&node);
            maxx = maxx.max(node_x + node_w);
            maxy = maxy.max(node_y + node_h);

            let labels: Vec<ElkLabelRef> = {
                let mut node_mut = node.borrow_mut();
                node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect()
            };
            for label in labels {
                if let Some(pos) = with_label_properties_mut(&label, |props| {
                    props.get_property(FixedLayouterOptions::POSITION)
                }) {
                    set_label_location(&label, &pos);
                }
                let (label_x, label_y, label_w, label_h) = label_bounds(&label);
                maxx = maxx.max(node_x + label_x + label_w);
                maxy = maxy.max(node_y + label_y + label_h);
            }

            let ports: Vec<ElkPortRef> = {
                let mut node_mut = node.borrow_mut();
                node_mut.ports().iter().cloned().collect()
            };
            for port in ports {
                if let Some(pos) = with_port_properties_mut(&port, |props| {
                    props.get_property(FixedLayouterOptions::POSITION)
                }) {
                    set_port_location(&port, &pos);
                }
                let (port_x, port_y, port_w, port_h) = port_bounds(&port);
                let abs_port_x = node_x + port_x;
                let abs_port_y = node_y + port_y;
                maxx = maxx.max(abs_port_x + port_w);
                maxy = maxy.max(abs_port_y + port_h);

                let port_labels: Vec<ElkLabelRef> = {
                    let mut port_mut = port.borrow_mut();
                    port_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .labels()
                        .iter()
                        .cloned()
                        .collect()
                };
                for label in port_labels {
                    if let Some(pos) = with_label_properties_mut(&label, |props| {
                        props.get_property(FixedLayouterOptions::POSITION)
                    }) {
                        set_label_location(&label, &pos);
                    }
                    let (label_x, label_y, label_w, label_h) = label_bounds(&label);
                    maxx = maxx.max(abs_port_x + label_x + label_w);
                    maxy = maxy.max(abs_port_y + label_y + label_h);
                }
            }

            for edge in ElkGraphUtil::all_outgoing_edges(&node) {
                let maxv = process_edge(&edge, edge_routing);
                maxx = maxx.max(maxv.x);
                maxy = maxy.max(maxv.y);
            }

            for edge in ElkGraphUtil::all_incoming_edges(&node) {
                if let Some(source_parent) = first_endpoint_parent(&edge, true) {
                    if !Rc::ptr_eq(&source_parent, layout_graph) {
                        let maxv = process_edge(&edge, edge_routing);
                        maxx = maxx.max(maxv.x);
                        maxy = maxy.max(maxv.y);
                    }
                }
            }
        }

        if edge_routing == EdgeRouting::Orthogonal {
            let nodes: Vec<ElkNodeRef> = {
                let mut layout_mut = layout_graph.borrow_mut();
                layout_mut.children().iter().cloned().collect()
            };
            for node in nodes {
                for edge in ElkGraphUtil::all_outgoing_edges(&node) {
                    generate_junction_points(&edge);
                }
            }
        }

        let fixed_graph_size = with_node_properties_mut(layout_graph, |props| {
            props
                .get_property(FixedLayouterOptions::NODE_SIZE_FIXED_GRAPH_SIZE)
                .unwrap_or(false)
        });
        if !fixed_graph_size {
            let padding = with_node_properties_mut(layout_graph, |props| {
                props
                    .get_property(FixedLayouterOptions::PADDING)
                    .unwrap_or_else(ElkPadding::new)
            });
            let new_width = maxx + padding.left + padding.right;
            let new_height = maxy + padding.top + padding.bottom;
            ElkUtil::resize_node_with(layout_graph, new_width, new_height, true, true);
        }

        progress_monitor.done();
    }
}

impl AbstractLayoutProvider for FixedLayoutProvider {}

fn process_edge(edge: &ElkEdgeRef, _edge_routing: EdgeRouting) -> KVector {
    let source_parent = first_endpoint_parent(edge, true);
    let target_parent = first_endpoint_parent(edge, false);
    let same_hierarchy = match (source_parent, target_parent) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(&a, &b),
        _ => false,
    };

    let mut maxv = KVector::new();
    let bend_points = with_edge_properties_mut(edge, |props| {
        props.get_property(FixedLayouterOptions::BEND_POINTS)
    });

    if let Some(bend_points) = bend_points {
        if bend_points.len() >= 2 {
            let section = {
                let mut edge_mut = edge.borrow_mut();
                let sections = edge_mut.sections();
                if sections.is_empty() {
                    let edge_section = ElkGraphFactory::instance().create_elk_edge_section();
                    sections.add(edge_section.clone());
                } else if sections.len() > 1 {
                    sections.retain_last();
                }
                sections.get(0)
            };

            if let Some(section) = section {
                ElkUtil::apply_vector_chain(&bend_points, &section);
            }
        }
    }

    if same_hierarchy {
        let sections: Vec<_> = {
            let mut edge_mut = edge.borrow_mut();
            edge_mut.sections().iter().cloned().collect()
        };
        for section in sections {
            let points: Vec<_> = {
                let mut section_mut = section.borrow_mut();
                section_mut.bend_points().to_vec()
            };
            for point in points {
                let point_borrow = point.borrow();
                maxv.x = maxv.x.max(point_borrow.x());
                maxv.y = maxv.y.max(point_borrow.y());
            }
        }
    }

    let labels: Vec<ElkLabelRef> = {
        let mut edge_mut = edge.borrow_mut();
        edge_mut.element().labels().iter().cloned().collect()
    };
    for label in labels {
        if let Some(pos) = with_label_properties_mut(&label, |props| {
            props.get_property(FixedLayouterOptions::POSITION)
        }) {
            set_label_location(&label, &pos);
        }
        if same_hierarchy {
            let (label_x, label_y, label_w, label_h) = label_bounds(&label);
            maxv.x = maxv.x.max(label_x + label_w);
            maxv.y = maxv.y.max(label_y + label_h);
        }
    }

    maxv
}

fn generate_junction_points(edge: &ElkEdgeRef) {
    let junction_points = ElkUtil::determine_junction_points(edge);
    with_edge_properties_mut(edge, |props| {
        if junction_points.is_empty() {
            props.set_property(CoreOptions::JUNCTION_POINTS, None);
        } else {
            props.set_property(CoreOptions::JUNCTION_POINTS, Some(junction_points));
        }
    });
}

fn first_endpoint_parent(edge: &ElkEdgeRef, is_source: bool) -> Option<ElkNodeRef> {
    let shape = {
        let edge_borrow = edge.borrow();
        let list = if is_source {
            edge_borrow.sources_ro()
        } else {
            edge_borrow.targets_ro()
        };
        list.get(0)
    };
    shape
        .and_then(|shape| ElkGraphUtil::connectable_shape_to_node(&shape))
        .and_then(|node| node.borrow().parent())
}

fn set_node_location(node: &ElkNodeRef, pos: &KVector) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    shape.set_x(pos.x);
    shape.set_y(pos.y);
}

fn set_port_location(port: &ElkPortRef, pos: &KVector) {
    let mut port_mut = port.borrow_mut();
    let shape = port_mut.connectable().shape();
    shape.set_x(pos.x);
    shape.set_y(pos.y);
}

fn set_label_location(label: &ElkLabelRef, pos: &KVector) {
    let mut label_mut = label.borrow_mut();
    let shape = label_mut.shape();
    shape.set_x(pos.x);
    shape.set_y(pos.y);
}

fn node_bounds(node: &ElkNodeRef) -> (f64, f64, f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}

fn port_bounds(port: &ElkPortRef) -> (f64, f64, f64, f64) {
    let mut port_mut = port.borrow_mut();
    let shape = port_mut.connectable().shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}

fn label_bounds(label: &ElkLabelRef) -> (f64, f64, f64, f64) {
    let mut label_mut = label.borrow_mut();
    let shape = label_mut.shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}

fn with_node_properties_mut<R>(
    node: &ElkNodeRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut node_mut = node.borrow_mut();
    f(node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut())
}

fn with_port_properties_mut<R>(
    port: &ElkPortRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut port_mut = port.borrow_mut();
    f(port_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut())
}

fn with_label_properties_mut<R>(
    label: &ElkLabelRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut label_mut = label.borrow_mut();
    f(label_mut.shape().graph_element().properties_mut())
}

fn with_edge_properties_mut<R>(
    edge: &ElkEdgeRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut edge_mut = edge.borrow_mut();
    f(edge_mut.element().properties_mut())
}
