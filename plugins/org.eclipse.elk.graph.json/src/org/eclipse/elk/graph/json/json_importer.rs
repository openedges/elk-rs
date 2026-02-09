use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use serde_json::{Map, Value};

use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{CoreOptions, EdgeCoords, ShapeCoords};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{
    IPropertyValueProxy, MapPropertyHolder, Property, PropertyValue,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdge, ElkEdgeRef, ElkEdgeSection, ElkEdgeSectionRef,
    ElkGraphElementRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

use super::json_adapter::{JsonAdapter, JsonId};
use super::json_import_exception::{JsonImportError, JsonImportException};

type JsonResult<T> = Result<T, JsonImportError>;

#[derive(Default)]
pub struct JsonImporter {
    node_id_map: HashMap<JsonId, ElkNodeRef>,
    port_id_map: HashMap<JsonId, ElkPortRef>,
    edge_id_map: HashMap<JsonId, ElkEdgeRef>,
    edge_section_id_map: HashMap<JsonId, ElkEdgeSectionRef>,
    node_ptr_map: HashMap<usize, String>,
    port_ptr_map: HashMap<usize, String>,
    edge_ptr_map: HashMap<usize, String>,
    edge_section_ptr_map: HashMap<usize, String>,
    label_ptr_map: HashMap<usize, String>,
    node_ids_by_key: HashMap<usize, JsonId>,
    port_ids_by_key: HashMap<usize, JsonId>,
    edge_ids_by_key: HashMap<usize, JsonId>,
    edge_section_ids_by_key: HashMap<usize, JsonId>,
    edge_original_parent: HashMap<usize, ElkNodeRef>,
    global_x_map: HashMap<usize, f64>,
    global_y_map: HashMap<usize, f64>,
    shape_coords_map: HashMap<usize, ShapeCoords>,
    edge_coords_map: HashMap<usize, EdgeCoords>,
    input_model: Option<Rc<RefCell<Value>>>,
}

impl JsonImporter {
    pub fn new() -> Self {
        JsonImporter::default()
    }

    pub fn transform(&mut self, graph: Value) -> JsonResult<ElkNodeRef> {
        let shared = Rc::new(RefCell::new(graph));
        self.transform_shared(shared)
    }

    pub fn transform_shared(&mut self, graph: Rc<RefCell<Value>>) -> JsonResult<ElkNodeRef> {
        self.clear_maps();
        self.input_model = Some(graph.clone());

        let root_json = graph.borrow();
        let root_obj = root_json.as_object().ok_or_else(|| {
            JsonImportError::from(JsonImportException::new(
                "Top-level element of the graph must be a json object.",
            ))
        })?;

        let root = self.transform_node(root_obj, "", None)?;
        self.transform_edges(root_obj)?;

        Ok(root)
    }

    pub fn input_model(&self) -> Option<Rc<RefCell<Value>>> {
        self.input_model.clone()
    }

    pub fn transfer_layout(&mut self, graph: &ElkNodeRef) -> JsonResult<()> {
        let model = self
            .input_model
            .clone()
            .ok_or_else(|| JsonImportException::new("No input model available."))?;
        let mut root = model.borrow_mut();

        self.transfer_nodes_and_ports(graph, &mut root)?;
        self.transfer_edges_and_labels(graph, &mut root)?;

        Ok(())
    }

    fn clear_maps(&mut self) {
        self.node_id_map.clear();
        self.port_id_map.clear();
        self.edge_id_map.clear();
        self.edge_section_id_map.clear();
        self.node_ptr_map.clear();
        self.port_ptr_map.clear();
        self.edge_ptr_map.clear();
        self.edge_section_ptr_map.clear();
        self.label_ptr_map.clear();
        self.node_ids_by_key.clear();
        self.port_ids_by_key.clear();
        self.edge_ids_by_key.clear();
        self.edge_section_ids_by_key.clear();
        self.edge_original_parent.clear();
        self.global_x_map.clear();
        self.global_y_map.clear();
        self.shape_coords_map.clear();
        self.edge_coords_map.clear();
    }

    fn transform_child_nodes(&mut self, json_node: &Map<String, Value>, parent: &ElkNodeRef, pointer: &str) -> JsonResult<()> {
        if let Some(children) = JsonAdapter::opt_json_array(json_node, "children") {
            for (index, child_value) in children.iter().enumerate() {
                if let Some(child_obj) = child_value.as_object() {
                    let child_ptr = pointer_index(pointer_key(pointer, "children"), index);
                    self.transform_node(child_obj, &child_ptr, Some(parent.clone()))?;
                }
            }
        }
        Ok(())
    }

    fn transform_node(
        &mut self,
        json_node: &Map<String, Value>,
        pointer: &str,
        parent: Option<ElkNodeRef>,
    ) -> JsonResult<ElkNodeRef> {
        let node = ElkGraphUtil::create_node(parent);
        self.register_node(&node, json_node, pointer.to_string())?;

        if let Some(identifier) = JsonAdapter::id_save(json_node)? {
            node.borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .set_identifier(Some(identifier));
        }

        with_node_properties_mut(&node, |props| self.transform_properties(json_node, props))?;
        with_node_properties_mut(&node, |props| self.transform_individual_spacings(json_node, props))?;
        self.transform_shape_layout(json_node, node.borrow_mut().connectable().shape())?;
        self.transform_ports(json_node, &node, pointer)?;
        self.transform_labels(json_node, ElkGraphElementRef::Node(node.clone()), pointer)?;
        self.transform_child_nodes(json_node, &node, pointer)?;

        Ok(node)
    }

    fn transform_edges(&mut self, json_obj: &Map<String, Value>) -> JsonResult<()> {
        let node_id = JsonAdapter::get_id(json_obj)?;
        let node = self.node_id_map.get(&node_id).cloned().ok_or_else(|| {
            JsonImportError::from(JsonImportException::new(
                "Unable to find elk node for json object. Panic!",
            ))
        })?;
        let node_ptr = self
            .node_ptr_map
            .get(&node_key(&node))
            .cloned()
            .unwrap_or_default();

        if let Some(edges) = JsonAdapter::opt_json_array(json_obj, "edges") {
            for (index, edge_value) in edges.iter().enumerate() {
                if let Some(edge_obj) = edge_value.as_object() {
                    let edge_ptr = pointer_index(pointer_key(node_ptr.as_str(), "edges"), index);
                    if JsonAdapter::has_key(edge_obj, "sources") || JsonAdapter::has_key(edge_obj, "targets") {
                        self.transform_edge(edge_obj, &node, &edge_ptr)?;
                    } else {
                        self.transform_primitive_edge(edge_obj, &node, &edge_ptr)?;
                    }
                    if let Some(edge) = self.edge_id_map.get(&JsonAdapter::get_id(edge_obj)?) {
                        ElkGraphUtil::update_containment(edge);
                    }
                }
            }
        }

        if let Some(children) = JsonAdapter::opt_json_array(json_obj, "children") {
            for child_value in children.iter() {
                if let Some(child_obj) = child_value.as_object() {
                    self.transform_edges(child_obj)?;
                }
            }
        }

        Ok(())
    }

    fn transform_primitive_edge(
        &mut self,
        json_edge: &Map<String, Value>,
        parent: &ElkNodeRef,
        pointer: &str,
    ) -> JsonResult<()> {
        let edge = ElkGraphUtil::create_edge(Some(parent.clone()));
        self.register_edge(&edge, json_edge, pointer.to_string())?;

        if let Some(identifier) = JsonAdapter::id_save(json_edge)? {
            edge.borrow_mut().element().set_identifier(Some(identifier));
        }

        let source_id_value = json_edge
            .get("source")
            .ok_or_else(|| JsonImportException::new("An edge must have a source node."))?;
        let source_id = JsonAdapter::as_id(source_id_value)?;
        let source_node = self.node_id_map.get(&source_id).cloned().ok_or_else(|| {
            JsonImportError::from(JsonImportException::new(
                "An edge must have a source node.",
            ))
        })?;
        let source_port = if let Some(port_value) = json_edge.get("sourcePort") {
            let port_id = JsonAdapter::as_id(port_value)?;
            self.port_id_map.get(&port_id).cloned()
        } else {
            None
        };

        if let Some(port) = source_port.as_ref() {
            if port.borrow().parent().map(|p| !Rc::ptr_eq(&p, &source_node)).unwrap_or(true) {
                return Err(JsonImportError::from(JsonImportException::new(
                    "The source port of an edge must be a port of the edge's source node.",
                )));
            }
            ElkEdge::add_source(&edge, ElkConnectableShapeRef::Port(port.clone()));
        } else {
            ElkEdge::add_source(&edge, ElkConnectableShapeRef::Node(source_node.clone()));
        }

        let target_id_value = json_edge
            .get("target")
            .ok_or_else(|| JsonImportException::new("An edge must have a target node."))?;
        let target_id = JsonAdapter::as_id(target_id_value)?;
        let target_node = self.node_id_map.get(&target_id).cloned().ok_or_else(|| {
            JsonImportError::from(JsonImportException::new(
                "An edge must have a target node.",
            ))
        })?;
        let target_port = if let Some(port_value) = json_edge.get("targetPort") {
            let port_id = JsonAdapter::as_id(port_value)?;
            self.port_id_map.get(&port_id).cloned()
        } else {
            None
        };

        if let Some(port) = target_port.as_ref() {
            if port.borrow().parent().map(|p| !Rc::ptr_eq(&p, &target_node)).unwrap_or(true) {
                return Err(JsonImportError::from(JsonImportException::new(
                    "The target port of an edge must be a port of the edge's target node.",
                )));
            }
            ElkEdge::add_target(&edge, ElkConnectableShapeRef::Port(port.clone()));
        } else {
            ElkEdge::add_target(&edge, ElkConnectableShapeRef::Node(target_node.clone()));
        }

        if edge.borrow().sources_ro().is_empty() || edge.borrow().targets_ro().is_empty() {
            return Err(JsonImportError::from(JsonImportException::new(
                "An edge must have at least one source and one target.",
            )));
        }

        with_edge_properties_mut(&edge, |props| self.transform_properties(json_edge, props))?;
        self.transform_primitive_edge_layout(json_edge, &edge)?;
        self.transform_labels(json_edge, ElkGraphElementRef::Edge(edge.clone()), pointer)?;

        Ok(())
    }

    fn transform_primitive_edge_layout(
        &mut self,
        json_edge: &Map<String, Value>,
        edge: &ElkEdgeRef,
    ) -> JsonResult<()> {
        let has_source = JsonAdapter::has_key(json_edge, "sourcePoint");
        let has_target = JsonAdapter::has_key(json_edge, "targetPoint");
        let has_bend = JsonAdapter::has_key(json_edge, "bendPoints");
        if !has_source && !has_target && !has_bend {
            return Ok(());
        }

        let section = create_edge_section(edge);

        if let Some(source_point) = JsonAdapter::opt_json_object(json_edge, "sourcePoint") {
            if let Some(x) = JsonAdapter::opt_double(source_point, "x")? {
                section.borrow_mut().set_start_x(double_value_valid(Some(x)));
            }
            if let Some(y) = JsonAdapter::opt_double(source_point, "y")? {
                section.borrow_mut().set_start_y(double_value_valid(Some(y)));
            }
        }

        if let Some(target_point) = JsonAdapter::opt_json_object(json_edge, "targetPoint") {
            if let Some(x) = JsonAdapter::opt_double(target_point, "x")? {
                section.borrow_mut().set_end_x(double_value_valid(Some(x)));
            }
            if let Some(y) = JsonAdapter::opt_double(target_point, "y")? {
                section.borrow_mut().set_end_y(double_value_valid(Some(y)));
            }
        }

        if let Some(bend_points) = JsonAdapter::opt_json_array(json_edge, "bendPoints") {
            for bend_value in bend_points {
                if let Some(bend_obj) = bend_value.as_object() {
                    let x = double_value_valid(JsonAdapter::opt_double(bend_obj, "x")?);
                    let y = double_value_valid(JsonAdapter::opt_double(bend_obj, "y")?);
                    create_bend_point(&section, x, y);
                }
            }
        }

        Ok(())
    }

    fn transform_edge(
        &mut self,
        json_edge: &Map<String, Value>,
        parent: &ElkNodeRef,
        pointer: &str,
    ) -> JsonResult<()> {
        let edge = ElkGraphUtil::create_edge(Some(parent.clone()));
        self.register_edge(&edge, json_edge, pointer.to_string())?;

        if let Some(identifier) = JsonAdapter::id_save(json_edge)? {
            edge.borrow_mut().element().set_identifier(Some(identifier));
        }

        if let Some(sources) = JsonAdapter::opt_json_array(json_edge, "sources") {
            for source_value in sources {
                let source_id = JsonAdapter::as_id(source_value)?;
                let shape = self.shape_by_id(&source_id)?;
                ElkEdge::add_source(&edge, shape);
            }
        }

        if let Some(targets) = JsonAdapter::opt_json_array(json_edge, "targets") {
            for target_value in targets {
                let target_id = JsonAdapter::as_id(target_value)?;
                let shape = self.shape_by_id(&target_id)?;
                ElkEdge::add_target(&edge, shape);
            }
        }

        if edge.borrow().sources_ro().is_empty() || edge.borrow().targets_ro().is_empty() {
            return Err(JsonImportError::from(JsonImportException::new(
                "An edge must have at least one source and one target.",
            )));
        }

        with_edge_properties_mut(&edge, |props| self.transform_properties(json_edge, props))?;
        self.transform_edge_sections(json_edge, &edge)?;
        self.transform_labels(json_edge, ElkGraphElementRef::Edge(edge.clone()), pointer)?;

        Ok(())
    }

    fn transform_edge_sections(&mut self, json_edge: &Map<String, Value>, edge: &ElkEdgeRef) -> JsonResult<()> {
        let mut incoming_identifiers: HashMap<usize, Vec<JsonId>> = HashMap::new();
        let mut outgoing_identifiers: HashMap<usize, Vec<JsonId>> = HashMap::new();
        let mut section_refs: HashMap<usize, ElkEdgeSectionRef> = HashMap::new();

        if let Some(sections) = JsonAdapter::opt_json_array(json_edge, "sections") {
            for (index, section_value) in sections.iter().enumerate() {
                if let Some(section_obj) = section_value.as_object() {
                    let section_ptr = pointer_index(pointer_key(
                        self.edge_ptr_map.get(&edge_key(edge)).map(|s| s.as_str()).unwrap_or(""),
                        "sections",
                    ), index);
                    let section = create_edge_section(edge);
                    self.register_edge_section(&section, section_obj, section_ptr)?;
                    if let Some(identifier) = JsonAdapter::id_save(section_obj)? {
                        section.borrow_mut().set_identifier(Some(identifier));
                    }

                    self.fill_edge_section_coordinates(section_obj, &section)?;

                    if let Some(shape_id) = JsonAdapter::opt_string(section_obj, "incomingShape")? {
                        let shape = self.shape_by_id(&JsonId::String(shape_id))?;
                        section.borrow_mut().set_incoming_shape(Some(shape));
                    }

                    if let Some(shape_id) = JsonAdapter::opt_string(section_obj, "outgoingShape")? {
                        let shape = self.shape_by_id(&JsonId::String(shape_id))?;
                        section.borrow_mut().set_outgoing_shape(Some(shape));
                    }

                    if let Some(incoming_sections) = JsonAdapter::opt_json_array(section_obj, "incomingSections") {
                        let entry = incoming_identifiers.entry(edge_section_key(&section)).or_default();
                        for id_value in incoming_sections {
                            entry.push(JsonAdapter::as_id(id_value)?);
                        }
                    }

                    if let Some(outgoing_sections) = JsonAdapter::opt_json_array(section_obj, "outgoingSections") {
                        let entry = outgoing_identifiers.entry(edge_section_key(&section)).or_default();
                        for id_value in outgoing_sections {
                            entry.push(JsonAdapter::as_id(id_value)?);
                        }
                    }

                    self.transform_properties(section_obj, section.borrow_mut().properties_mut())?;

                    section_refs.insert(edge_section_key(&section), section.clone());
                }
            }
        }

        for (section_key, ids) in incoming_identifiers {
            let section = section_refs.get(&section_key).unwrap();
            let mut resolved = Vec::new();
            for id in ids {
                let referenced = self.edge_section_id_map.get(&id).ok_or_else(|| {
                    JsonImportError::from(JsonImportException::new(
                        "Referenced edge section does not exist.",
                    ))
                })?;
                resolved.push(referenced.clone());
            }
            section.borrow_mut().set_incoming_sections(resolved);
        }

        for (section_key, ids) in outgoing_identifiers {
            let section = section_refs.get(&section_key).unwrap();
            let mut resolved = Vec::new();
            for id in ids {
                let referenced = self.edge_section_id_map.get(&id).ok_or_else(|| {
                    JsonImportError::from(JsonImportException::new(
                        "Referenced edge section does not exist.",
                    ))
                })?;
                resolved.push(referenced.clone());
            }
            section.borrow_mut().set_outgoing_sections(resolved);
        }

        let (is_connected, is_hyperedge, single_section) = {
            let mut edge_mut = edge.borrow_mut();
            let is_connected = edge_mut.is_connected();
            let is_hyperedge = edge_mut.is_hyperedge();
            let single_section = if edge_mut.sections().len() == 1 {
                edge_mut.sections().get(0)
            } else {
                None
            };
            (is_connected, is_hyperedge, single_section)
        };

        if is_connected && !is_hyperedge {
            if let Some(section) = single_section {
                let needs_shapes = {
                    let section_ref = section.borrow();
                    section_ref.incoming_shape().is_none() && section_ref.outgoing_shape().is_none()
                };
                if needs_shapes {
                    let (source, target) = {
                        let edge_ref = edge.borrow();
                        let source = edge_ref.sources_ro().get(0).unwrap();
                        let target = edge_ref.targets_ro().get(0).unwrap();
                        (source, target)
                    };
                    section.borrow_mut().set_incoming_shape(Some(source));
                    section.borrow_mut().set_outgoing_shape(Some(target));
                }
            }
        }

        Ok(())
    }

    fn fill_edge_section_coordinates(
        &mut self,
        json_section: &Map<String, Value>,
        section: &ElkEdgeSectionRef,
    ) -> JsonResult<()> {
        let start_point = JsonAdapter::opt_json_object(json_section, "startPoint")
            .ok_or_else(|| JsonImportException::new("All edge sections need a start point."))?;
        let end_point = JsonAdapter::opt_json_object(json_section, "endPoint")
            .ok_or_else(|| JsonImportException::new("All edge sections need an end point."))?;

        if let Some(x) = JsonAdapter::opt_double(start_point, "x")? {
            section.borrow_mut().set_start_x(double_value_valid(Some(x)));
        }
        if let Some(y) = JsonAdapter::opt_double(start_point, "y")? {
            section.borrow_mut().set_start_y(double_value_valid(Some(y)));
        }
        if let Some(x) = JsonAdapter::opt_double(end_point, "x")? {
            section.borrow_mut().set_end_x(double_value_valid(Some(x)));
        }
        if let Some(y) = JsonAdapter::opt_double(end_point, "y")? {
            section.borrow_mut().set_end_y(double_value_valid(Some(y)));
        }

        if let Some(bend_points) = JsonAdapter::opt_json_array(json_section, "bendPoints") {
            for bend_value in bend_points {
                if let Some(bend_obj) = bend_value.as_object() {
                    let x = double_value_valid(JsonAdapter::opt_double(bend_obj, "x")?);
                    let y = double_value_valid(JsonAdapter::opt_double(bend_obj, "y")?);
                    create_bend_point(section, x, y);
                }
            }
        }

        Ok(())
    }

    fn transform_properties(
        &self,
        json_obj: &Map<String, Value>,
        properties: &mut MapPropertyHolder,
    ) -> JsonResult<()> {
        let layout_options = JsonAdapter::opt_json_object(json_obj, "layoutOptions")
            .or_else(|| JsonAdapter::opt_json_object(json_obj, "properties"));

        if let Some(options) = layout_options {
            for (key, value) in options {
                let value_str = JsonAdapter::string_val(value)?;
                self.set_option(properties, key, &value_str);
            }
        }
        Ok(())
    }

    fn transform_individual_spacings(
        &self,
        json_obj: &Map<String, Value>,
        properties: &mut MapPropertyHolder,
    ) -> JsonResult<()> {
        let Some(spacings_obj) = JsonAdapter::opt_json_object(json_obj, "individualSpacings") else {
            return Ok(());
        };

        let mut individual = properties
            .get_property(CoreOptions::SPACING_INDIVIDUAL)
            .unwrap_or_default();

        for (key, value) in spacings_obj {
            let value_str = JsonAdapter::string_val(value)?;
            self.set_option(individual.properties_mut(), key, &value_str);
        }

        properties.set_property(CoreOptions::SPACING_INDIVIDUAL, Some(individual));
        Ok(())
    }

    fn set_option(&self, properties: &mut MapPropertyHolder, id: &str, value: &str) {
        if id == CoreOptions::DIRECTION.id() {
            if let Some(direction) = parse_direction_option(value) {
                properties.set_property(CoreOptions::DIRECTION, Some(direction));
                return;
            }
        }

        let option_data = LayoutMetaDataService::get_instance().get_option_data_by_suffix(id);
        if let Some(option_data) = option_data {
            if let Some(parsed) = option_data.parse_value(value) {
                properties.set_property_any(option_data.id(), Some(parsed));
            }
        }
    }

    fn transform_labels(
        &mut self,
        json_obj: &Map<String, Value>,
        parent: ElkGraphElementRef,
        pointer: &str,
    ) -> JsonResult<()> {
        if let Some(labels) = JsonAdapter::opt_json_array(json_obj, "labels") {
            for (index, label_value) in labels.iter().enumerate() {
                if let Some(label_obj) = label_value.as_object() {
                    let label_ptr = pointer_index(pointer_key(pointer, "labels"), index);
                    let text = JsonAdapter::opt_string(label_obj, "text")?.unwrap_or_default();
                    let label = ElkGraphUtil::create_label_with_text(text, Some(parent.clone()));
                    if JsonAdapter::has_key(label_obj, "id") {
                        if let Some(identifier) = JsonAdapter::id_save(label_obj)? {
                            label.borrow_mut()
                                .shape()
                                .graph_element()
                                .set_identifier(Some(identifier));
                        }
                    }
                    self.label_ptr_map.insert(label_key(&label), label_ptr);

                    with_label_properties_mut(&label, |props| self.transform_properties(label_obj, props))?;
                    self.transform_shape_layout(label_obj, label.borrow_mut().shape())?;
                }
            }
        }
        Ok(())
    }

    fn transform_ports(
        &mut self,
        json_node: &Map<String, Value>,
        parent: &ElkNodeRef,
        pointer: &str,
    ) -> JsonResult<()> {
        if let Some(ports) = JsonAdapter::opt_json_array(json_node, "ports") {
            for (index, port_value) in ports.iter().enumerate() {
                if let Some(port_obj) = port_value.as_object() {
                    let port_ptr = pointer_index(pointer_key(pointer, "ports"), index);
                    self.transform_port(port_obj, parent, &port_ptr)?;
                }
            }
        }
        Ok(())
    }

    fn transform_port(
        &mut self,
        json_port: &Map<String, Value>,
        parent: &ElkNodeRef,
        pointer: &str,
    ) -> JsonResult<()> {
        let port = ElkGraphUtil::create_port(Some(parent.clone()));
        self.register_port(&port, json_port, pointer.to_string())?;

        if let Some(identifier) = JsonAdapter::id_save(json_port)? {
            port.borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .set_identifier(Some(identifier));
        }

        with_port_properties_mut(&port, |props| self.transform_properties(json_port, props))?;
        self.transform_shape_layout(json_port, port.borrow_mut().connectable().shape())?;
        self.transform_labels(json_port, ElkGraphElementRef::Port(port.clone()), pointer)?;
        Ok(())
    }

    fn transform_shape_layout(
        &self,
        json_obj: &Map<String, Value>,
        shape: &mut org_eclipse_elk_graph::org::eclipse::elk::graph::ElkShape,
    ) -> JsonResult<()> {
        if let Some(x) = JsonAdapter::opt_double(json_obj, "x")? {
            shape.set_x(double_value_valid(Some(x)));
        }
        if let Some(y) = JsonAdapter::opt_double(json_obj, "y")? {
            shape.set_y(double_value_valid(Some(y)));
        }
        if let Some(width) = JsonAdapter::opt_double(json_obj, "width")? {
            shape.set_width(double_value_valid(Some(width)));
        }
        if let Some(height) = JsonAdapter::opt_double(json_obj, "height")? {
            shape.set_height(double_value_valid(Some(height)));
        }
        Ok(())
    }

    fn shape_by_id(&self, id: &JsonId) -> JsonResult<ElkConnectableShapeRef> {
        if let Some(node) = self.node_id_map.get(id) {
            return Ok(ElkConnectableShapeRef::Node(node.clone()));
        }
        if let Some(port) = self.port_id_map.get(id) {
            return Ok(ElkConnectableShapeRef::Port(port.clone()));
        }
        Err(JsonImportError::from(JsonImportException::new(
            format!("Referenced shape does not exist: {}", id.as_string()),
        )))
    }

    fn register_node(
        &mut self,
        node: &ElkNodeRef,
        json_obj: &Map<String, Value>,
        pointer: String,
    ) -> JsonResult<()> {
        let id = JsonAdapter::get_id(json_obj)?;
        self.node_id_map.insert(id.clone(), node.clone());
        self.node_ids_by_key.insert(node_key(node), id);
        self.node_ptr_map.insert(node_key(node), pointer);
        Ok(())
    }

    fn register_port(
        &mut self,
        port: &ElkPortRef,
        json_obj: &Map<String, Value>,
        pointer: String,
    ) -> JsonResult<()> {
        let id = JsonAdapter::get_id(json_obj)?;
        self.port_id_map.insert(id.clone(), port.clone());
        self.port_ids_by_key.insert(port_key(port), id);
        self.port_ptr_map.insert(port_key(port), pointer);
        Ok(())
    }

    fn register_edge(
        &mut self,
        edge: &ElkEdgeRef,
        json_obj: &Map<String, Value>,
        pointer: String,
    ) -> JsonResult<()> {
        let id = JsonAdapter::get_id(json_obj)?;
        self.edge_id_map.insert(id.clone(), edge.clone());
        self.edge_ids_by_key.insert(edge_key(edge), id);
        self.edge_ptr_map.insert(edge_key(edge), pointer);
        if let Some(parent) = edge.borrow().containing_node() {
            self.edge_original_parent.insert(edge_key(edge), parent);
        }
        Ok(())
    }

    fn register_edge_section(
        &mut self,
        section: &ElkEdgeSectionRef,
        json_obj: &Map<String, Value>,
        pointer: String,
    ) -> JsonResult<()> {
        let id = JsonAdapter::get_id(json_obj)?;
        self.edge_section_id_map.insert(id.clone(), section.clone());
        self.edge_section_ids_by_key
            .insert(edge_section_key(section), id);
        self.edge_section_ptr_map
            .insert(edge_section_key(section), pointer);
        Ok(())
    }

    fn transfer_nodes_and_ports(&mut self, root: &ElkNodeRef, json_root: &mut Value) -> JsonResult<()> {
        let mut stack = vec![root.clone()];
        while let Some(node) = stack.pop() {
            self.transfer_layout_node(&node, json_root)?;
            let ports = node_ports(&node);
            for port in ports {
                self.transfer_layout_port(&port, json_root)?;
            }
            let children = node_children(&node);
            for child in children {
                stack.push(child);
            }
        }
        Ok(())
    }

    fn transfer_edges_and_labels(
        &mut self,
        root: &ElkNodeRef,
        json_root: &mut Value,
    ) -> JsonResult<()> {
        let mut stack = vec![root.clone()];
        while let Some(node) = stack.pop() {
            for label in node_labels(&node) {
                self.transfer_layout_label(&label, json_root)?;
            }
            for port in node_ports(&node) {
                for label in port_labels(&port) {
                    self.transfer_layout_label(&label, json_root)?;
                }
            }
            for edge in node_edges(&node) {
                self.transfer_layout_edge(&edge, json_root)?;
                for label in edge_labels(&edge) {
                    self.transfer_layout_label(&label, json_root)?;
                }
            }
            for child in node_children(&node) {
                stack.push(child);
            }
        }
        Ok(())
    }

    fn transfer_layout_node(&mut self, node: &ElkNodeRef, json_root: &mut Value) -> JsonResult<()> {
        let pointer = self
            .node_ptr_map
            .get(&node_key(node))
            .ok_or_else(|| JsonImportException::new("Node did not exist in input."))?
            .clone();
        let json_obj = json_object_mut(json_root, &pointer)?;
        self.record_global_coords_node(node);
        self.record_coordinate_modes(ElkGraphElementRef::Node(node.clone()));
        let parent = self.json_parent(ElkGraphElementRef::Node(node.clone()));
        {
            let mut node_mut = node.borrow_mut();
            self.transfer_shape_layout_to_json(node_mut.connectable().shape(), json_obj, parent);
        }
        Ok(())
    }

    fn transfer_layout_port(&mut self, port: &ElkPortRef, json_root: &mut Value) -> JsonResult<()> {
        let pointer = self
            .port_ptr_map
            .get(&port_key(port))
            .ok_or_else(|| JsonImportException::new("Port did not exist in input."))?
            .clone();
        let json_obj = json_object_mut(json_root, &pointer)?;
        self.record_global_coords_port(port);
        self.record_coordinate_modes(ElkGraphElementRef::Port(port.clone()));
        let parent = self.json_parent(ElkGraphElementRef::Port(port.clone()));
        {
            let mut port_mut = port.borrow_mut();
            self.transfer_shape_layout_to_json(port_mut.connectable().shape(), json_obj, parent);
        }
        Ok(())
    }

    fn transfer_layout_edge(&mut self, edge: &ElkEdgeRef, json_root: &mut Value) -> JsonResult<()> {
        let pointer = self
            .edge_ptr_map
            .get(&edge_key(edge))
            .ok_or_else(|| JsonImportException::new("Edge did not exist in input."))?
            .clone();
        self.record_coordinate_modes(ElkGraphElementRef::Edge(edge.clone()));

        let edge_id = self.edge_ids_by_key.get(&edge_key(edge)).map(|id| id.as_string()).unwrap_or_default();

        let sections = {
            let mut edge_mut = edge.borrow_mut();
            let list = edge_mut.sections();
            (0..list.len()).filter_map(|i| list.get(i)).collect::<Vec<_>>()
        };

        let mut json_sections = Vec::new();
        if !sections.is_empty() {
            for (index, section) in sections.iter().enumerate() {
                let mut json_section_obj = if let Some(pointer) = self.edge_section_ptr_map.get(&edge_section_key(section)) {
                    json_object_clone(json_root, pointer).unwrap_or_default()
                } else {
                    let mut obj = Map::new();
                    obj.insert("id".to_string(), Value::String(format!("{edge_id}_s{index}")));
                    obj
                };

                let (start_x, start_y, end_x, end_y, bend_points, incoming_shape, outgoing_shape, incoming_sections, outgoing_sections) = {
                    let mut section_ref = section.borrow_mut();
                    let bend_points = section_ref
                        .bend_points()
                        .iter()
                        .map(|bend| {
                            let bend_ref = bend.borrow();
                            (bend_ref.x(), bend_ref.y())
                        })
                        .collect::<Vec<_>>();
                    let incoming_shape = section_ref.incoming_shape();
                    let outgoing_shape = section_ref.outgoing_shape();
                    let incoming_sections = section_ref.incoming_sections();
                    let outgoing_sections = section_ref.outgoing_sections();
                    (
                        section_ref.start_x(),
                        section_ref.start_y(),
                        section_ref.end_x(),
                        section_ref.end_y(),
                        bend_points,
                        incoming_shape,
                        outgoing_shape,
                        incoming_sections,
                        outgoing_sections,
                    )
                };

                let start_point = point_object(
                    self.adjust_edge_x(edge, start_x)?,
                    self.adjust_edge_y(edge, start_y)?,
                );
                json_section_obj.insert("startPoint".to_string(), start_point);

                let end_point = point_object(
                    self.adjust_edge_x(edge, end_x)?,
                    self.adjust_edge_y(edge, end_y)?,
                );
                json_section_obj.insert("endPoint".to_string(), end_point);

                if !bend_points.is_empty() {
                    let mut json_bends = Vec::new();
                    for (x, y) in bend_points {
                        json_bends.push(point_object(
                            self.adjust_edge_x(edge, x)?,
                            self.adjust_edge_y(edge, y)?,
                        ));
                    }
                    json_section_obj.insert("bendPoints".to_string(), Value::Array(json_bends));
                }

                if let Some(shape) = incoming_shape {
                    if let Some(id) = self.id_by_element(&shape) {
                        json_section_obj.insert("incomingShape".to_string(), json_id_value(&id));
                    }
                }

                if let Some(shape) = outgoing_shape {
                    if let Some(id) = self.id_by_element(&shape) {
                        json_section_obj.insert("outgoingShape".to_string(), json_id_value(&id));
                    }
                }

                if !incoming_sections.is_empty() {
                    let mut json_incoming = Vec::new();
                    for sec in incoming_sections {
                        if let Some(id) = self.id_by_edge_section(&sec) {
                            json_incoming.push(json_id_value(&id));
                        }
                    }
                    json_section_obj.insert("incomingSections".to_string(), Value::Array(json_incoming));
                }

                if !outgoing_sections.is_empty() {
                    let mut json_outgoing = Vec::new();
                    for sec in outgoing_sections {
                        if let Some(id) = self.id_by_edge_section(&sec) {
                            json_outgoing.push(json_id_value(&id));
                        }
                    }
                    json_section_obj.insert("outgoingSections".to_string(), Value::Array(json_outgoing));
                }

                json_sections.push(Value::Object(json_section_obj));
            }
        }

        let junction_points = with_edge_properties_mut(edge, |props| {
            if props.has_property(CoreOptions::JUNCTION_POINTS) {
                props.get_property(CoreOptions::JUNCTION_POINTS)
            } else {
                None
            }
        });

        let container_id = if let Some(parent) = self.edge_original_parent.get(&edge_key(edge)) {
            if self.edge_coords_mode(&ElkGraphElementRef::Node(parent.clone())) == EdgeCoords::Container {
                parent
                    .borrow_mut()
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(|v| v.to_string())
            } else {
                None
            }
        } else {
            None
        };

        let json_obj = json_object_mut(json_root, &pointer)?;

        if !json_sections.is_empty() {
            json_obj.remove("sections");
            json_obj.insert("sections".to_string(), Value::Array(json_sections));
        }

        if let Some(jps) = junction_points {
            if !jps.is_empty() {
                let mut json_jps = Vec::new();
                for jp in jps.iter() {
                    json_jps.push(point_object(
                        self.adjust_edge_x(edge, jp.x)?,
                        self.adjust_edge_y(edge, jp.y)?,
                    ));
                }
                json_obj.insert("junctionPoints".to_string(), Value::Array(json_jps));
            }
        }

        if let Some(container_id) = container_id {
            json_obj.insert("container".to_string(), Value::String(container_id));
        }

        Ok(())
    }

    fn transfer_layout_label(&mut self, label: &ElkLabelRef, json_root: &mut Value) -> JsonResult<()> {
        let pointer = self
            .label_ptr_map
            .get(&label_key(label))
            .ok_or_else(|| JsonImportException::new("Label did not exist in input."))?
            .clone();
        let json_obj = json_object_mut(json_root, &pointer)?;
        self.record_global_coords_label(label);
        self.record_coordinate_modes(ElkGraphElementRef::Label(label.clone()));
        let parent = self.json_parent(ElkGraphElementRef::Label(label.clone()));
        {
            let mut label_mut = label.borrow_mut();
            self.transfer_shape_layout_to_json(label_mut.shape(), json_obj, parent);
        }
        Ok(())
    }

    fn transfer_shape_layout_to_json(
        &self,
        shape: &mut org_eclipse_elk_graph::org::eclipse::elk::graph::ElkShape,
        json_obj: &mut Map<String, Value>,
        parent: Option<ElkGraphElementRef>,
    ) {
        let x = shape.x();
        let y = shape.y();
        let adjusted_x = parent.as_ref().map(|p| self.adjust_parent_x(p, x)).unwrap_or(x);
        let adjusted_y = parent.as_ref().map(|p| self.adjust_parent_y(p, y)).unwrap_or(y);
        json_obj.insert("x".to_string(), Value::Number(f64_to_number(adjusted_x)));
        json_obj.insert("y".to_string(), Value::Number(f64_to_number(adjusted_y)));
        json_obj.insert("width".to_string(), Value::Number(f64_to_number(shape.width())));
        json_obj.insert("height".to_string(), Value::Number(f64_to_number(shape.height())));
    }

    fn adjust_parent_x(&self, parent: &ElkGraphElementRef, x: f64) -> f64 {
        match parent {
            ElkGraphElementRef::Edge(edge) => self.adjust_edge_x(edge, x).unwrap_or(x),
            _ => match self.shape_coords_mode(parent) {
                ShapeCoords::Root => x + self.global_x(parent),
                _ => x,
            },
        }
    }

    fn adjust_parent_y(&self, parent: &ElkGraphElementRef, y: f64) -> f64 {
        match parent {
            ElkGraphElementRef::Edge(edge) => self.adjust_edge_y(edge, y).unwrap_or(y),
            _ => match self.shape_coords_mode(parent) {
                ShapeCoords::Root => y + self.global_y(parent),
                _ => y,
            },
        }
    }

    fn adjust_edge_x(&self, edge: &ElkEdgeRef, x: f64) -> JsonResult<f64> {
        let mode = self
            .edge_original_parent
            .get(&edge_key(edge))
            .map(|parent| self.edge_coords_mode(&ElkGraphElementRef::Node(parent.clone())))
            .unwrap_or(EdgeCoords::Container);
        let containing = edge
            .borrow()
            .containing_node()
            .ok_or_else(|| JsonImportException::new("Edge has no container."))?;
        match mode {
            EdgeCoords::Root => Ok(x + self.global_x(&ElkGraphElementRef::Node(containing))),
            EdgeCoords::Parent => {
                let original = self
                    .edge_original_parent
                    .get(&edge_key(edge))
                    .ok_or_else(|| JsonImportException::new("Edge has no original parent."))?;
                Ok(x + self.global_x(&ElkGraphElementRef::Node(containing))
                    - self.global_x(&ElkGraphElementRef::Node(original.clone())))
            }
            _ => Ok(x),
        }
    }

    fn adjust_edge_y(&self, edge: &ElkEdgeRef, y: f64) -> JsonResult<f64> {
        let mode = self
            .edge_original_parent
            .get(&edge_key(edge))
            .map(|parent| self.edge_coords_mode(&ElkGraphElementRef::Node(parent.clone())))
            .unwrap_or(EdgeCoords::Container);
        let containing = edge
            .borrow()
            .containing_node()
            .ok_or_else(|| JsonImportException::new("Edge has no container."))?;
        match mode {
            EdgeCoords::Root => Ok(y + self.global_y(&ElkGraphElementRef::Node(containing))),
            EdgeCoords::Parent => {
                let original = self
                    .edge_original_parent
                    .get(&edge_key(edge))
                    .ok_or_else(|| JsonImportException::new("Edge has no original parent."))?;
                Ok(y + self.global_y(&ElkGraphElementRef::Node(containing))
                    - self.global_y(&ElkGraphElementRef::Node(original.clone())))
            }
            _ => Ok(y),
        }
    }

    fn id_by_element(&self, shape: &ElkConnectableShapeRef) -> Option<JsonId> {
        match shape {
            ElkConnectableShapeRef::Node(node) => self.node_ids_by_key.get(&node_key(node)).cloned(),
            ElkConnectableShapeRef::Port(port) => self.port_ids_by_key.get(&port_key(port)).cloned(),
        }
    }

    fn id_by_edge_section(&self, section: &ElkEdgeSectionRef) -> Option<JsonId> {
        self.edge_section_ids_by_key
            .get(&edge_section_key(section))
            .cloned()
    }

    fn record_coordinate_modes(&mut self, element: ElkGraphElementRef) {
        let parent = self.json_parent(element.clone());

        let mut shape_mode = get_property_optional(&element, CoreOptions::JSON_SHAPE_COORDS)
            .unwrap_or(ShapeCoords::Inherit);
        if shape_mode == ShapeCoords::Inherit {
            shape_mode = parent
                .as_ref()
                .map(|p| self.shape_coords_mode(p))
                .unwrap_or(ShapeCoords::Parent);
        }
        self.shape_coords_map.insert(element_key(&element), shape_mode);

        let mut edge_mode = get_property_optional(&element, CoreOptions::JSON_EDGE_COORDS)
            .unwrap_or(EdgeCoords::Inherit);
        if edge_mode == EdgeCoords::Inherit {
            edge_mode = parent
                .as_ref()
                .map(|p| self.edge_coords_mode(p))
                .unwrap_or(EdgeCoords::Container);
        }
        self.edge_coords_map.insert(element_key(&element), edge_mode);
    }

    fn record_global_coords_node(&mut self, node: &ElkNodeRef) {
        let parent = node.borrow().parent();
        let ancestor = parent
            .as_ref()
            .and_then(|p| self.shape_ancestor(&ElkGraphElementRef::Node(p.clone())));
        let dx = ancestor.as_ref().map(|a| self.global_x(a)).unwrap_or(0.0);
        let dy = ancestor.as_ref().map(|a| self.global_y(a)).unwrap_or(0.0);
        let x = node.borrow_mut().connectable().shape().x() + dx;
        let y = node.borrow_mut().connectable().shape().y() + dy;
        self.global_x_map.insert(node_key(node), x);
        self.global_y_map.insert(node_key(node), y);
    }

    fn record_global_coords_port(&mut self, port: &ElkPortRef) {
        let parent = port.borrow().parent();
        let ancestor = parent
            .as_ref()
            .and_then(|p| self.shape_ancestor(&ElkGraphElementRef::Node(p.clone())));
        let dx = ancestor.as_ref().map(|a| self.global_x(a)).unwrap_or(0.0);
        let dy = ancestor.as_ref().map(|a| self.global_y(a)).unwrap_or(0.0);
        let x = port.borrow_mut().connectable().shape().x() + dx;
        let y = port.borrow_mut().connectable().shape().y() + dy;
        self.global_x_map.insert(port_key(port), x);
        self.global_y_map.insert(port_key(port), y);
    }

    fn record_global_coords_label(&mut self, label: &ElkLabelRef) {
        let parent = label.borrow().parent();
        let ancestor = parent
            .as_ref()
            .and_then(|p| self.shape_ancestor(p));
        let dx = ancestor.as_ref().map(|a| self.global_x(a)).unwrap_or(0.0);
        let dy = ancestor.as_ref().map(|a| self.global_y(a)).unwrap_or(0.0);
        let x = label.borrow_mut().shape().x() + dx;
        let y = label.borrow_mut().shape().y() + dy;
        self.global_x_map.insert(label_key(label), x);
        self.global_y_map.insert(label_key(label), y);
    }

    fn shape_ancestor(&self, element: &ElkGraphElementRef) -> Option<ElkGraphElementRef> {
        match element {
            ElkGraphElementRef::Edge(edge) => self
                .edge_original_parent
                .get(&edge_key(edge))
                .map(|parent| ElkGraphElementRef::Node(parent.clone())),
            _ => Some(element.clone()),
        }
    }

    fn json_parent(&self, element: ElkGraphElementRef) -> Option<ElkGraphElementRef> {
        match element {
            ElkGraphElementRef::Node(node) => node.borrow().parent().map(ElkGraphElementRef::Node),
            ElkGraphElementRef::Port(port) => port.borrow().parent().map(ElkGraphElementRef::Node),
            ElkGraphElementRef::Edge(edge) => self
                .edge_original_parent
                .get(&edge_key(&edge))
                .cloned()
                .map(ElkGraphElementRef::Node),
            ElkGraphElementRef::Label(label) => label.borrow().parent(),
        }
    }

    fn global_x(&self, element: &ElkGraphElementRef) -> f64 {
        match element {
            ElkGraphElementRef::Node(node) => self.global_x_map.get(&node_key(node)).copied().unwrap_or(0.0),
            ElkGraphElementRef::Port(port) => self.global_x_map.get(&port_key(port)).copied().unwrap_or(0.0),
            ElkGraphElementRef::Label(label) => self.global_x_map.get(&label_key(label)).copied().unwrap_or(0.0),
            ElkGraphElementRef::Edge(_) => 0.0,
        }
    }

    fn global_y(&self, element: &ElkGraphElementRef) -> f64 {
        match element {
            ElkGraphElementRef::Node(node) => self.global_y_map.get(&node_key(node)).copied().unwrap_or(0.0),
            ElkGraphElementRef::Port(port) => self.global_y_map.get(&port_key(port)).copied().unwrap_or(0.0),
            ElkGraphElementRef::Label(label) => self.global_y_map.get(&label_key(label)).copied().unwrap_or(0.0),
            ElkGraphElementRef::Edge(_) => 0.0,
        }
    }

    fn shape_coords_mode(&self, element: &ElkGraphElementRef) -> ShapeCoords {
        self.shape_coords_map
            .get(&element_key(element))
            .copied()
            .unwrap_or(ShapeCoords::Parent)
    }

    fn edge_coords_mode(&self, element: &ElkGraphElementRef) -> EdgeCoords {
        self.edge_coords_map
            .get(&element_key(element))
            .copied()
            .unwrap_or(EdgeCoords::Container)
    }
}

fn double_value_valid(value: Option<f64>) -> f64 {
    match value {
        Some(value) if value.is_finite() => value,
        _ => 0.0,
    }
}

fn node_key(node: &ElkNodeRef) -> usize {
    Rc::as_ptr(node) as usize
}

fn port_key(port: &ElkPortRef) -> usize {
    Rc::as_ptr(port) as usize
}

fn edge_key(edge: &ElkEdgeRef) -> usize {
    Rc::as_ptr(edge) as usize
}

fn edge_section_key(section: &ElkEdgeSectionRef) -> usize {
    Rc::as_ptr(section) as usize
}

fn label_key(label: &ElkLabelRef) -> usize {
    Rc::as_ptr(label) as usize
}

fn element_key(element: &ElkGraphElementRef) -> usize {
    match element {
        ElkGraphElementRef::Node(node) => node_key(node),
        ElkGraphElementRef::Port(port) => port_key(port),
        ElkGraphElementRef::Edge(edge) => edge_key(edge),
        ElkGraphElementRef::Label(label) => label_key(label),
    }
}

fn pointer_key(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        format!("/{}", escape_pointer_segment(key))
    } else {
        format!("{}/{}", parent, escape_pointer_segment(key))
    }
}

fn pointer_index(parent: String, index: usize) -> String {
    format!("{}/{}", parent, index)
}

fn parse_direction_option(value: &str) -> Option<Direction> {
    let trimmed = value.trim();
    if let Ok(index) = trimmed.parse::<usize>() {
        return match index {
            0 => Some(Direction::Undefined),
            1 => Some(Direction::Right),
            2 => Some(Direction::Left),
            3 => Some(Direction::Down),
            4 => Some(Direction::Up),
            _ => None,
        };
    }

    match trimmed.to_ascii_uppercase().as_str() {
        "UNDEFINED" => Some(Direction::Undefined),
        "RIGHT" => Some(Direction::Right),
        "LEFT" => Some(Direction::Left),
        "DOWN" => Some(Direction::Down),
        "UP" => Some(Direction::Up),
        _ => None,
    }
}

fn escape_pointer_segment(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn json_object_mut<'a>(root: &'a mut Value, pointer: &str) -> JsonResult<&'a mut Map<String, Value>> {
    let value = if pointer.is_empty() {
        root
    } else {
        root.pointer_mut(pointer).ok_or_else(|| {
            JsonImportError::from(JsonImportException::new("JSON pointer did not resolve."))
        })?
    };
    value.as_object_mut().ok_or_else(|| {
        JsonImportError::from(JsonImportException::new("Expected JSON object."))
    })
}

fn json_object_clone(root: &Value, pointer: &str) -> Option<Map<String, Value>> {
    let value = if pointer.is_empty() {
        root
    } else {
        root.pointer(pointer)?
    };
    value.as_object().cloned()
}

fn point_object(x: f64, y: f64) -> Value {
    let mut obj = Map::new();
    obj.insert("x".to_string(), Value::Number(f64_to_number(x)));
    obj.insert("y".to_string(), Value::Number(f64_to_number(y)));
    Value::Object(obj)
}

fn json_id_value(id: &JsonId) -> Value {
    match id {
        JsonId::String(text) => Value::String(text.clone()),
        JsonId::Int(value) => Value::Number((*value).into()),
    }
}

fn f64_to_number(value: f64) -> serde_json::Number {
    serde_json::Number::from_f64(value).unwrap_or_else(|| serde_json::Number::from(0))
}

fn create_edge_section(edge: &ElkEdgeRef) -> ElkEdgeSectionRef {
    let section = ElkEdgeSection::new();
    ElkEdgeSection::set_parent(&section, Some(edge.clone()));
    section
}

fn create_bend_point(section: &ElkEdgeSectionRef, x: f64, y: f64) {
    let bend = org_eclipse_elk_graph::org::eclipse::elk::graph::ElkBendPoint::new();
    bend.borrow_mut().set_x(x);
    bend.borrow_mut().set_y(y);
    section.borrow_mut().bend_points().push(bend);
}

fn get_property_optional<T: Clone + Send + Sync + 'static>(
    element: &ElkGraphElementRef,
    property: &Property<T>,
) -> Option<T> {
    match element {
        ElkGraphElementRef::Node(node) => {
            let mut node_mut = node.borrow_mut();
            resolve_property_optional(
                node_mut.connectable().shape().graph_element().properties(),
                property,
            )
        }
        ElkGraphElementRef::Port(port) => {
            let mut port_mut = port.borrow_mut();
            resolve_property_optional(
                port_mut.connectable().shape().graph_element().properties(),
                property,
            )
        }
        ElkGraphElementRef::Edge(edge) => {
            let mut edge_mut = edge.borrow_mut();
            resolve_property_optional(edge_mut.element().properties(), property)
        }
        ElkGraphElementRef::Label(label) => {
            let mut label_mut = label.borrow_mut();
            resolve_property_optional(label_mut.shape().graph_element().properties(), property)
        }
    }
}

fn resolve_property_optional<T: Clone + Send + Sync + 'static>(
    props: &MapPropertyHolder,
    property: &Property<T>,
) -> Option<T> {
    props.get_all_properties().get(property.id()).and_then(|value| match value {
        PropertyValue::Resolved(value) => value.downcast_ref::<T>().cloned(),
        PropertyValue::Proxy(proxy) => resolve_proxy::<T>(proxy.as_ref(), property.id()),
    })
}

fn resolve_proxy<T: Clone + Send + Sync + 'static>(
    proxy: &dyn IPropertyValueProxy,
    property_id: &str,
) -> Option<T> {
    proxy
        .resolve_value(property_id)
        .and_then(|resolved| resolved.downcast_ref::<T>().cloned())
}

fn with_node_properties_mut<R>(node: &ElkNodeRef, f: impl FnOnce(&mut MapPropertyHolder) -> R) -> R {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    f(props)
}

fn with_port_properties_mut<R>(port: &ElkPortRef, f: impl FnOnce(&mut MapPropertyHolder) -> R) -> R {
    let mut port_mut = port.borrow_mut();
    let props = port_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    f(props)
}

fn with_edge_properties_mut<R>(edge: &ElkEdgeRef, f: impl FnOnce(&mut MapPropertyHolder) -> R) -> R {
    let mut edge_mut = edge.borrow_mut();
    let props = edge_mut.element().properties_mut();
    f(props)
}

fn with_label_properties_mut<R>(label: &ElkLabelRef, f: impl FnOnce(&mut MapPropertyHolder) -> R) -> R {
    let mut label_mut = label.borrow_mut();
    let props = label_mut.shape().graph_element().properties_mut();
    f(props)
}

fn node_children(node: &ElkNodeRef) -> Vec<ElkNodeRef> {
    node.borrow_mut()
        .children()
        .iter()
        .cloned()
        .collect()
}

fn node_ports(node: &ElkNodeRef) -> Vec<ElkPortRef> {
    node.borrow_mut().ports().iter().cloned().collect()
}

fn node_labels(node: &ElkNodeRef) -> Vec<ElkLabelRef> {
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .labels()
        .iter()
        .cloned()
        .collect()
}

fn node_edges(node: &ElkNodeRef) -> Vec<ElkEdgeRef> {
    node.borrow_mut()
        .contained_edges()
        .iter()
        .cloned()
        .collect()
}

fn port_labels(port: &ElkPortRef) -> Vec<ElkLabelRef> {
    port.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .labels()
        .iter()
        .cloned()
        .collect()
}

fn edge_labels(edge: &ElkEdgeRef) -> Vec<ElkLabelRef> {
    edge.borrow_mut()
        .element()
        .labels()
        .iter()
        .cloned()
        .collect()
}
