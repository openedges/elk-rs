use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use org_eclipse_elk_core::org::eclipse::elk::core::math::{kvector::KVector, kvector_chain::KVectorChain};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkEdgeRef, ElkGraphElementRef, ElkGraphFactory, ElkNodeRef, ElkPortRef,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

use crate::org::eclipse::elk::alg::disco::graph::{DCElement, DCElementRef, DCExtension, DCGraph, DCDirection};
use crate::org::eclipse::elk::alg::disco::options::DisCoOptions;
use crate::org::eclipse::elk::alg::disco::transform::ElkGraphComponentsProcessor;
use crate::org::eclipse::elk::alg::disco::transform::IGraphTransformer;

const HALF_PI: f64 = std::f64::consts::PI / 2.0;

pub struct ElkGraphTransformer {
    parent: Option<ElkNodeRef>,
    element_mapping: HashMap<usize, (ElkGraphElementRef, DCElementRef)>,
    incoming_extensions: HashMap<usize, (ElkEdgeRef, DCExtension)>,
    outgoing_extensions: HashMap<usize, (ElkEdgeRef, DCExtension)>,
    transformed_graph: Option<DCGraph>,
    component_spacing: f64,
}

impl ElkGraphTransformer {
    pub fn new(component_spacing: f64) -> Self {
        ElkGraphTransformer {
            parent: None,
            element_mapping: HashMap::new(),
            incoming_extensions: HashMap::new(),
            outgoing_extensions: HashMap::new(),
            transformed_graph: None,
            component_spacing,
        }
    }

    fn import_elk_shape(
        &mut self,
        element: ElkGraphElementRef,
        consider_when_applying_offset: bool,
        offset_x: f64,
        offset_y: f64,
    ) -> DCElementRef {
        let (x, y, width, height, props) = match &element {
            ElkGraphElementRef::Node(node) => {
                let mut node_mut = node.borrow_mut();
                let shape = node_mut.connectable().shape();
                (
                    shape.x(),
                    shape.y(),
                    shape.width(),
                    shape.height(),
                    shape.graph_element().properties().clone(),
                )
            }
            ElkGraphElementRef::Port(port) => {
                let mut port_mut = port.borrow_mut();
                let shape = port_mut.connectable().shape();
                (
                    shape.x(),
                    shape.y(),
                    shape.width(),
                    shape.height(),
                    shape.graph_element().properties().clone(),
                )
            }
            ElkGraphElementRef::Label(label) => {
                let mut label_mut = label.borrow_mut();
                let shape = label_mut.shape();
                (
                    shape.x(),
                    shape.y(),
                    shape.width(),
                    shape.height(),
                    shape.graph_element().properties().clone(),
                )
            }
            ElkGraphElementRef::Edge(_) => {
                panic!("Use import_elk_edge for ElkEdge instances.");
            }
        };

        let half_component_spacing = self.component_spacing / 2.0;
        let x0 = x + offset_x - half_component_spacing;
        let y0 = y + offset_y - half_component_spacing;
        let x1 = x0 + width + self.component_spacing;
        let y1 = y0 + height + self.component_spacing;

        let mut coords = KVectorChain::new();
        coords.add_vector(KVector::with_values(x0, y0));
        coords.add_vector(KVector::with_values(x0, y1));
        coords.add_vector(KVector::with_values(x1, y1));
        coords.add_vector(KVector::with_values(x1, y0));

        let element_ref = Arc::new(Mutex::new(DCElement::new(coords)));
        {
            let mut element_guard = element_ref.lock().expect("dc element lock");
            element_guard.copy_properties(&props);
        }

        if consider_when_applying_offset {
            self.element_mapping
                .insert(element_id(&element), (element.clone(), element_ref.clone()));
        }

        element_ref
    }

    fn import_elk_edge(&mut self, edge: &ElkEdgeRef, new_component: &mut Vec<DCElementRef>) -> DCElementRef {
        let edge_section = first_edge_section(edge, false, false);
        let points = ElkUtil::create_vector_chain(&edge_section).to_array();
        let thickness = get_edge_thickness(edge);
        let contour = get_contour(&points, thickness + self.component_spacing);

        let shape = Arc::new(Mutex::new(DCElement::new(contour)));
        {
            let props = {
                let mut edge_mut = edge.borrow_mut();
                edge_mut.element().properties().clone()
            };
            let mut shape_guard = shape.lock().expect("dc element lock");
            shape_guard.copy_properties(&props);
        }
        self.element_mapping
            .insert(edge_id(edge), (ElkGraphElementRef::Edge(edge.clone()), shape.clone()));
        new_component.push(shape.clone());

        let labels = {
            let mut edge_mut = edge.borrow_mut();
            edge_mut.element().labels().iter().cloned().collect::<Vec<_>>()
        };
        for label in labels {
            let component_label =
                self.import_elk_shape(ElkGraphElementRef::Label(label), true, 0.0, 0.0);
            new_component.push(component_label);
        }

        shape
    }

    fn import_extension(
        &mut self,
        edge: &ElkEdgeRef,
        new_component: &mut Vec<DCElementRef>,
        outgoing_extension: bool,
    ) {
        let edge_section = first_edge_section(edge, false, false);
        let mut points = ElkUtil::create_vector_chain(&edge_section);
        if outgoing_extension {
            points = KVectorChain::reverse(&points);
        }

        let thickness = get_edge_thickness(edge);
        let shape: DCElementRef;
        if points.size() > 2 {
            let fixed_edge_points = points.to_array_from(1);
            let contour = get_contour(&fixed_edge_points, thickness + self.component_spacing);
            shape = Arc::new(Mutex::new(DCElement::new(contour)));
            {
                let props = {
                    let mut edge_mut = edge.borrow_mut();
                    edge_mut.element().properties().clone()
                };
                let mut shape_guard = shape.lock().expect("dc element lock");
                shape_guard.copy_properties(&props);
            }
            new_component.push(shape.clone());
        } else {
            let node = if outgoing_extension {
                get_source_node(edge)
            } else {
                get_target_node(edge)
            };
            let key = element_id(&ElkGraphElementRef::Node(node));
            shape = self
                .element_mapping
                .get(&key)
                .map(|(_, value)| value.clone())
                .expect("missing node shape for extension");
        }

        let outer_point = points.get_first();
        let inner_point = points.get(1);
        let ext_parent = if outgoing_extension {
            get_target_node(edge)
        } else {
            get_source_node(edge)
        };
        let dir = nearest_side(&outer_point, &ext_parent);
        let mut extension_width = thickness + self.component_spacing;
        let middle_pos: KVector;

        if dir.is_horizontal() {
            extension_width += (outer_point.y - inner_point.y).abs();
            middle_pos = KVector::with_values(inner_point.x, (inner_point.y + outer_point.y) / 2.0);
        } else {
            extension_width += (outer_point.x - inner_point.x).abs();
            middle_pos = KVector::with_values((inner_point.x + outer_point.x) / 2.0, inner_point.y);
        }

        let bounds = { shape.lock().expect("dc element lock").get_bounds() };
        let extension = DCExtension::new(&bounds, dir, &middle_pos, extension_width);
        {
            let mut shape_guard = shape.lock().expect("dc element lock");
            shape_guard.add_extension(extension.clone());
        }

        if outgoing_extension {
            self.outgoing_extensions
                .insert(edge_id(edge), (edge.clone(), extension.clone()));
        } else {
            self.incoming_extensions
                .insert(edge_id(edge), (edge.clone(), extension.clone()));
        }
        self.element_mapping
            .insert(edge_id(edge), (ElkGraphElementRef::Edge(edge.clone()), shape.clone()));

        let labels = {
            let mut edge_mut = edge.borrow_mut();
            edge_mut.element().labels().iter().cloned().collect::<Vec<_>>()
        };
        for label in labels {
            let component_label =
                self.import_elk_shape(ElkGraphElementRef::Label(label), true, 0.0, 0.0);
            new_component.push(component_label);
        }
    }

    fn import_elk_edges(&mut self, edges: &[ElkEdgeRef], new_component: &mut Vec<DCElementRef>) {
        for edge in edges {
            if self.element_mapping.contains_key(&edge_id(edge)) {
                continue;
            }

            let source_node = get_source_node(edge);
            let target_node = get_target_node(edge);
            let source_parent = source_node.borrow().parent();
            let target_parent = target_node.borrow().parent();

            let same_parent = match (source_parent, target_parent) {
                (Some(sp), Some(tp)) => Rc::ptr_eq(&sp, &tp),
                (None, None) => true,
                _ => false,
            };

            if same_parent {
                self.import_elk_edge(edge, new_component);
            } else {
                let target_parent = target_node.borrow().parent();
                if let Some(target_parent) = target_parent {
                    if Rc::ptr_eq(&source_node, &target_parent) {
                        if !self.incoming_extensions.contains_key(&edge_id(edge))
                            && self
                                .element_mapping
                                .contains_key(&element_id(&ElkGraphElementRef::Node(target_node)))
                        {
                            self.import_extension(edge, new_component, false);
                        }
                        continue;
                    }
                }

                if !self.outgoing_extensions.contains_key(&edge_id(edge))
                    && self
                        .element_mapping
                        .contains_key(&element_id(&ElkGraphElementRef::Node(source_node)))
                {
                    self.import_extension(edge, new_component, true);
                }
            }
        }
    }

    pub fn graph(&self) -> Option<&DCGraph> {
        self.transformed_graph.as_ref()
    }
}

impl Default for ElkGraphTransformer {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl IGraphTransformer<ElkNodeRef> for ElkGraphTransformer {
    fn import_graph(&mut self, graph: &ElkNodeRef) -> &mut DCGraph {
        self.parent = Some(graph.clone());
        self.element_mapping.clear();
        self.incoming_extensions.clear();
        self.outgoing_extensions.clear();

        let components = ElkGraphComponentsProcessor::split(graph);
        let mut result: Vec<Vec<DCElementRef>> = Vec::new();

        for component in components {
            let mut sub_result: Vec<DCElementRef> = Vec::new();
            let mut edge_ids: HashSet<usize> = HashSet::new();
            let mut edges: Vec<ElkEdgeRef> = Vec::new();

            for node in component {
                let component_node = self.import_elk_shape(
                    ElkGraphElementRef::Node(node.clone()),
                    true,
                    0.0,
                    0.0,
                );
                sub_result.push(component_node.clone());

                let (node_x, node_y) = {
                    let mut node_mut = node.borrow_mut();
                    let shape = node_mut.connectable().shape();
                    (shape.x(), shape.y())
                };
                {
                    let mut elem_guard = component_node.lock().expect("dc element lock");
                    elem_guard.set_parent_coords(KVector::with_values(node_x, node_y));
                }

                let labels = {
                    let mut node_mut = node.borrow_mut();
                    node_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .labels()
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                };
                for label in labels {
                    let component_label =
                        self.import_elk_shape(ElkGraphElementRef::Label(label), false, node_x, node_y);
                    sub_result.push(component_label);
                }

                let ports = {
                    let mut node_mut = node.borrow_mut();
                    node_mut.ports().iter().cloned().collect::<Vec<_>>()
                };
                for port in ports {
                    let component_port =
                        self.import_elk_shape(ElkGraphElementRef::Port(port.clone()), false, node_x, node_y);
                    sub_result.push(component_port);

                    let (port_x, port_y) = {
                        let mut port_mut = port.borrow_mut();
                        let shape = port_mut.connectable().shape();
                        (shape.x() + node_x, shape.y() + node_y)
                    };
                    let labels = {
                        let mut port_mut = port.borrow_mut();
                        port_mut
                            .connectable()
                            .shape()
                            .graph_element()
                            .labels()
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                    };
                    for label in labels {
                        let component_label = self.import_elk_shape(
                            ElkGraphElementRef::Label(label),
                            false,
                            port_x,
                            port_y,
                        );
                        sub_result.push(component_label);
                    }
                }

                for edge in ElkGraphUtil::all_incident_edges(&node) {
                    let id = edge_id(&edge);
                    if edge_ids.insert(id) {
                        edges.push(edge);
                    }
                }
            }

            self.import_elk_edges(&edges, &mut sub_result);
            result.push(sub_result);
        }

        let mut transformed_graph = DCGraph::new(result, self.component_spacing / 2.0);
        let props = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties()
                .clone()
        };
        transformed_graph.copy_properties(&props);

        self.transformed_graph = Some(transformed_graph);
        self.transformed_graph.as_mut().expect("missing transformed graph")
    }

    fn apply_layout(&mut self) {
        let Some(parent) = self.parent.clone() else {
            return;
        };
        let Some(graph) = self.transformed_graph.as_mut() else {
            return;
        };

        let graph_dimensions = graph.get_dimensions();
        let new_width = graph_dimensions.x;
        let new_height = graph_dimensions.y;

        let (old_width, old_height) = {
            let mut parent_mut = parent.borrow_mut();
            let shape = parent_mut.connectable().shape();
            let old_width = shape.width();
            let old_height = shape.height();
            shape.set_dimensions(new_width, new_height);
            (old_width, old_height)
        };

        let x_factor = new_width / old_width;
        let y_factor = new_height / old_height;

        let labels = {
            let mut parent_mut = parent.borrow_mut();
            parent_mut
                .connectable()
                .shape()
                .graph_element()
                .labels()
                .iter()
                .cloned()
                .collect::<Vec<_>>()
        };
        for label in labels {
            let mut label_mut = label.borrow_mut();
            let shape = label_mut.shape();
            shape.set_x(shape.x() * x_factor);
            shape.set_y(shape.y() * y_factor);
        }

        let ports = {
            let mut parent_mut = parent.borrow_mut();
            parent_mut.ports().iter().cloned().collect::<Vec<_>>()
        };
        for port in ports {
            let mut port_mut = port.borrow_mut();
            let shape = port_mut.connectable().shape();
            let px = shape.x();
            let py = shape.y();
            if px > 0.0 {
                shape.set_x(px * x_factor);
            }
            if py > 0.0 {
                shape.set_y(py * y_factor);
            }
        }

        for (_id, (elem, poly)) in self.element_mapping.iter() {
            let offset = { poly.lock().expect("dc element lock").get_offset() };
            match elem {
                ElkGraphElementRef::Edge(edge) => {
                    let edge_section = first_edge_section(edge, false, false);
                    let mut points = ElkUtil::create_vector_chain(&edge_section);
                    points.offset(offset.x, offset.y);
                    ElkUtil::apply_vector_chain(&points, &edge_section);

                    let mut edge_mut = edge.borrow_mut();
                    let junction_points = edge_mut
                        .element()
                        .properties_mut()
                        .get_property(CoreOptions::JUNCTION_POINTS);
                    if let Some(mut junction_points) = junction_points {
                        for point in junction_points.iter_mut() {
                            point.add_values(offset.x, offset.y);
                        }
                        edge_mut
                            .element()
                            .properties_mut()
                            .set_property(CoreOptions::JUNCTION_POINTS, Some(junction_points));
                    }
                }
                ElkGraphElementRef::Node(node) => {
                    let mut node_mut = node.borrow_mut();
                    let shape = node_mut.connectable().shape();
                    shape.set_x(shape.x() + offset.x);
                    shape.set_y(shape.y() + offset.y);
                }
                ElkGraphElementRef::Port(port) => {
                    let mut port_mut = port.borrow_mut();
                    let shape = port_mut.connectable().shape();
                    shape.set_x(shape.x() + offset.x);
                    shape.set_y(shape.y() + offset.y);
                }
                ElkGraphElementRef::Label(label) => {
                    let mut label_mut = label.borrow_mut();
                    let shape = label_mut.shape();
                    shape.set_x(shape.x() + offset.x);
                    shape.set_y(shape.y() + offset.y);
                }
            }
        }

        let mut adjusted_ports: HashSet<usize> = HashSet::new();

        for (_edge_id, (edge, extension)) in self.incoming_extensions.iter() {
            let dir = extension.get_direction();
            let edge_section = first_edge_section(edge, false, false);
            let points = ElkUtil::create_vector_chain(&edge_section);
            let new_points = adjust_first_segment(&get_source_node(edge), &points, dir);
            ElkUtil::apply_vector_chain(&new_points, &edge_section);

            if let Some(port) = get_source_port(edge) {
                let pid = port_id(&port);
                if adjusted_ports.insert(pid) {
                    adjust_related_port(&port, &new_points.get_first(), dir);
                }
            }
        }

        for (_edge_id, (edge, extension)) in self.outgoing_extensions.iter() {
            let dir = extension.get_direction();
            let edge_section = first_edge_section(edge, false, false);
            let points = ElkUtil::create_vector_chain(&edge_section);
            let reversed = KVectorChain::reverse(&points);
            let mut new_points = adjust_first_segment(&get_target_node(edge), &reversed, dir);
            new_points = KVectorChain::reverse(&new_points);
            ElkUtil::apply_vector_chain(&new_points, &edge_section);

            if let Some(port) = get_target_port(edge) {
                let pid = port_id(&port);
                if adjusted_ports.insert(pid) {
                    adjust_related_port(&port, &new_points.get_last(), dir);
                }
            }
        }

    }
}

fn adjust_related_port(port: &ElkPortRef, edge_point: &KVector, dir: DCDirection) {
    let mut port_mut = port.borrow_mut();
    let shape = port_mut.connectable().shape();
    if dir.is_horizontal() {
        shape.set_y(edge_point.y - (shape.height() / 2.0));
    } else {
        shape.set_x(edge_point.x - (shape.width() / 2.0));
    }
}

fn adjust_first_segment(source: &ElkNodeRef, chain: &KVectorChain, dir: DCDirection) -> KVectorChain {
    let mut points = chain.to_array();
    if points.is_empty() {
        return KVectorChain::new();
    }

    let mut first_point = points.remove(0);
    let (width, height) = {
        let mut source_mut = source.borrow_mut();
        let shape = source_mut.connectable().shape();
        (shape.width(), shape.height())
    };
    match dir {
        DCDirection::North => first_point.y = 0.0,
        DCDirection::South => first_point.y = height,
        DCDirection::West => first_point.x = 0.0,
        DCDirection::East => first_point.x = width,
    }
    points.insert(0, first_point);
    KVectorChain::from_vectors(&points)
}

fn get_contour(edge_points: &[KVector], thickness: f64) -> KVectorChain {
    if edge_points.len() < 2 {
        return KVectorChain::new();
    }

    let mut ccw_points: Vec<KVector> = Vec::new();
    let mut cw_points: Vec<KVector> = Vec::new();
    let radius = thickness / 2.0;

    let mut current = edge_points[0];
    let mut successor = edge_points[1];
    let orth_points = get_orthogonal_points(current.x, current.y, successor.x, successor.y, radius);
    ccw_points.push(orth_points[0]);
    cw_points.push(orth_points[1]);

    for i in 2..edge_points.len() {
        let predecessor = current;
        current = successor;
        successor = edge_points[i];

        let orth_points = get_orthogonal_points(current.x, current.y, predecessor.x, predecessor.y, radius);
        ccw_points.push(orth_points[1]);
        cw_points.push(orth_points[0]);

        let orth_points = get_orthogonal_points(current.x, current.y, successor.x, successor.y, radius);
        ccw_points.push(orth_points[0]);
        cw_points.push(orth_points[1]);
    }

    let orth_points = get_orthogonal_points(successor.x, successor.y, current.x, current.y, radius);
    ccw_points.push(orth_points[1]);
    cw_points.push(orth_points[0]);

    let mut ccw_merged = KVectorChain::new();
    let mut cw_merged: Vec<KVector> = Vec::new();

    ccw_merged.add_vector(ccw_points[0]);
    let mut i = 1;
    while i + 2 < ccw_points.len() {
        let current_point = ccw_points[i];
        let intersection_point = compute_intersection(
            &ccw_points[i - 1],
            &current_point,
            &ccw_points[i + 1],
            &ccw_points[i + 2],
        );
        if !intersection_point.x.is_finite() || !intersection_point.y.is_finite() {
            ccw_merged.add_vector(current_point);
        } else {
            ccw_merged.add_vector(intersection_point);
        }
        i += 2;
    }
    ccw_merged.add_vector(ccw_points[ccw_points.len() - 1]);

    cw_merged.push(cw_points[0]);
    let mut i = 1;
    while i + 2 < cw_points.len() {
        let current_point = cw_points[i];
        let intersection_point = compute_intersection(
            &cw_points[i - 1],
            &current_point,
            &cw_points[i + 1],
            &cw_points[i + 2],
        );
        if !intersection_point.x.is_finite() || !intersection_point.y.is_finite() {
            cw_merged.push(current_point);
        } else {
            cw_merged.push(intersection_point);
        }
        i += 2;
    }
    cw_merged.push(cw_points[cw_points.len() - 1]);

    for point in cw_merged.into_iter().rev() {
        ccw_merged.add_vector(point);
    }

    ccw_merged
}

fn get_orthogonal_points(cur_x: f64, cur_y: f64, nxt_x: f64, nxt_y: f64, radius: f64) -> Vec<KVector> {
    let dif_x = nxt_x - cur_x;
    let dif_y = nxt_y - cur_y;

    let angle_radians = dif_x.atan2(dif_y);
    let orth_angle_ccw = angle_radians + HALF_PI;
    let orth_angle_cw = angle_radians - HALF_PI;

    let x_ccw = radius * orth_angle_ccw.sin() + cur_x;
    let y_ccw = radius * orth_angle_ccw.cos() + cur_y;
    let x_cw = radius * orth_angle_cw.sin() + cur_x;
    let y_cw = radius * orth_angle_cw.cos() + cur_y;

    vec![KVector::with_values(x_ccw, y_ccw), KVector::with_values(x_cw, y_cw)]
}

fn compute_intersection(p1: &KVector, p2: &KVector, p3: &KVector, p4: &KVector) -> KVector {
    let x1 = p1.x;
    let y1 = p1.y;
    let x2 = p2.x;
    let y2 = p2.y;
    let x3 = p3.x;
    let y3 = p3.y;
    let x4 = p4.x;
    let y4 = p4.y;

    let factor1 = x1 * y2 - y1 * x2;
    let factor2 = x3 * y4 - y3 * x4;
    let denominator = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);

    let x = (factor1 * (x3 - x4) - factor2 * (x1 - x2)) / denominator;
    let y = (factor1 * (y3 - y4) - factor2 * (y1 - y2)) / denominator;

    KVector::with_values(x, y)
}

fn nearest_side(point: &KVector, node: &ElkNodeRef) -> DCDirection {
    let mut result = DCDirection::North;

    let (width, height) = {
        let mut node_mut = node.borrow_mut();
        let shape = node_mut.connectable().shape();
        (shape.width(), shape.height())
    };

    let mut shortest_distance = point.y.abs();
    let mut distance = (height - point.y).abs();
    if distance < shortest_distance {
        shortest_distance = distance;
        result = DCDirection::South;
    }
    distance = point.x.abs();
    if distance < shortest_distance {
        shortest_distance = distance;
        result = DCDirection::West;
    }
    distance = (width - point.x).abs();
    if distance < shortest_distance {
        result = DCDirection::East;
    }
    result
}

fn first_edge_section(edge: &ElkEdgeRef, reset_section: bool, remove_other_sections: bool) -> org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeSectionRef {
    let mut edge_mut = edge.borrow_mut();
    let sections = edge_mut.sections();
    if sections.is_empty() {
        let section = ElkGraphFactory::instance().create_elk_edge_section();
        sections.add(section.clone());
        return section;
    }

    let section = sections.get(0).expect("edge section");
    if reset_section {
        let mut section_mut = section.borrow_mut();
        section_mut.bend_points().clear();
        section_mut.set_start_x(0.0);
        section_mut.set_start_y(0.0);
        section_mut.set_end_x(0.0);
        section_mut.set_end_y(0.0);
    }

    if remove_other_sections {
        sections.retain_last();
    }

    section
}

fn get_edge_thickness(edge: &ElkEdgeRef) -> f64 {
    let mut edge_mut = edge.borrow_mut();
    edge_mut
        .element()
        .properties_mut()
        .get_property(DisCoOptions::EDGE_THICKNESS)
        .unwrap_or(0.0)
}

fn get_source_node(edge: &ElkEdgeRef) -> ElkNodeRef {
    let (source, _target) = edge_endpoints(edge);
    ElkGraphUtil::connectable_shape_to_node(&source)
        .expect("Passed edge is not 'simple'.")
}

fn get_target_node(edge: &ElkEdgeRef) -> ElkNodeRef {
    let (_source, target) = edge_endpoints(edge);
    ElkGraphUtil::connectable_shape_to_node(&target)
        .expect("Passed edge is not 'simple'.")
}

fn get_source_port(edge: &ElkEdgeRef) -> Option<ElkPortRef> {
    let (source, _target) = edge_endpoints(edge);
    ElkGraphUtil::connectable_shape_to_port(&source)
}

fn get_target_port(edge: &ElkEdgeRef) -> Option<ElkPortRef> {
    let (_source, target) = edge_endpoints(edge);
    ElkGraphUtil::connectable_shape_to_port(&target)
}

fn edge_endpoints(
    edge: &ElkEdgeRef,
) -> (
    org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef,
    org_eclipse_elk_graph::org::eclipse::elk::graph::ElkConnectableShapeRef,
) {
    let edge_borrow = edge.borrow();
    if edge_borrow.sources_ro().len() != 1 || edge_borrow.targets_ro().len() != 1 {
        panic!("Passed edge is not 'simple'.");
    }
    let source = edge_borrow.sources_ro().get(0).expect("missing source").clone();
    let target = edge_borrow.targets_ro().get(0).expect("missing target").clone();
    (source, target)
}

fn element_id(element: &ElkGraphElementRef) -> usize {
    match element {
        ElkGraphElementRef::Node(node) => Rc::as_ptr(node) as usize,
        ElkGraphElementRef::Edge(edge) => Rc::as_ptr(edge) as usize,
        ElkGraphElementRef::Port(port) => Rc::as_ptr(port) as usize,
        ElkGraphElementRef::Label(label) => Rc::as_ptr(label) as usize,
    }
}

fn edge_id(edge: &ElkEdgeRef) -> usize {
    Rc::as_ptr(edge) as usize
}

fn port_id(port: &ElkPortRef) -> usize {
    Rc::as_ptr(port) as usize
}
