use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value};

use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{
    ElkMargin, ElkPadding, KVector, KVectorChain,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    Alignment, ContentAlignment, CoreOptions, Direction, EdgeCoords, EdgeLabelPlacement,
    EdgeRouting, EdgeType, HierarchyHandling, LabelSide, NodeLabelPlacement, PortAlignment,
    PortConstraints, PortLabelPlacement, PortSide, ShapeCoords, SizeConstraint, SizeOptions,
    TopdownNodeTypes,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    EnumSet, EnumSetType, IndividualSpacings,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{
    MapPropertyHolder, Property, PropertyValue,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkEdgeSectionRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

#[derive(Default)]
pub struct JsonExporter {
    node_id_map: HashMap<usize, String>,
    port_id_map: HashMap<usize, String>,
    edge_id_map: HashMap<usize, String>,
    edge_section_id_map: HashMap<usize, String>,
    node_ptr_map: HashMap<usize, String>,
    node_ids_used: HashSet<String>,
    port_ids_used: HashSet<String>,
    edge_ids_used: HashSet<String>,
    edge_section_ids_used: HashSet<String>,
    node_id_counter: i32,
    port_id_counter: i32,
    edge_id_counter: i32,
    edge_section_id_counter: i32,
    unique_counter: u64,
    omit_zero_pos: bool,
    omit_zero_dim: bool,
    omit_layout: bool,
    short_layout_option_keys: bool,
    omit_unknown_layout_options: bool,
}

impl JsonExporter {
    pub fn new() -> Self {
        JsonExporter::default()
    }

    pub fn set_options(
        &mut self,
        omit_zero_pos: bool,
        omit_zero_dim: bool,
        omit_layout: bool,
        short_layout_option_keys: bool,
        omit_unknown_layout_options: bool,
    ) {
        self.omit_zero_pos = omit_zero_pos;
        self.omit_zero_dim = omit_zero_dim;
        self.omit_layout = omit_layout;
        self.short_layout_option_keys = short_layout_option_keys;
        self.omit_unknown_layout_options = omit_unknown_layout_options;
    }

    pub fn export(&mut self, root: &ElkNodeRef) -> Value {
        self.init();
        let mut root_value = Value::Null;
        self.transform_node(root, "", &mut root_value);
        self.transform_edges(root, &mut root_value);
        root_value
    }

    fn init(&mut self) {
        self.node_id_map.clear();
        self.port_id_map.clear();
        self.edge_id_map.clear();
        self.edge_section_id_map.clear();
        self.node_ptr_map.clear();
        self.node_ids_used.clear();
        self.port_ids_used.clear();
        self.edge_ids_used.clear();
        self.edge_section_ids_used.clear();
        self.node_id_counter = 0;
        self.port_id_counter = 0;
        self.edge_id_counter = 0;
        self.edge_section_id_counter = 0;
        self.unique_counter = 0;
    }

    fn transform_node(&mut self, node: &ElkNodeRef, pointer: &str, out: &mut Value) {
        let mut obj = Map::new();
        let id = self.create_and_register_node(node, pointer);
        obj.insert("id".to_string(), Value::String(id));

        let labels = node_labels(node);
        if !labels.is_empty() {
            let mut labels_arr = Vec::new();
            for label in labels {
                self.transform_label(&label, &mut labels_arr);
            }
            obj.insert("labels".to_string(), Value::Array(labels_arr));
        }

        let ports = node_ports(node);
        if !ports.is_empty() {
            let mut ports_arr = Vec::new();
            for port in ports.iter() {
                self.transform_port(port, &mut ports_arr);
            }
            obj.insert("ports".to_string(), Value::Array(ports_arr));
        }

        let children = node_children(node);
        if !children.is_empty() {
            let mut children_arr = Vec::new();
            for (index, child) in children.iter().enumerate() {
                let child_ptr = pointer_index(pointer_key(pointer, "children"), index);
                let mut child_value = Value::Null;
                self.transform_node(child, &child_ptr, &mut child_value);
                children_arr.push(child_value);
            }
            obj.insert("children".to_string(), Value::Array(children_arr));
        }

        with_node_properties(node, |props| {
            self.transform_properties(props, &mut obj);
            self.transform_individual_spacings(props, &mut obj);
        });
        {
            let mut node_mut = node.borrow_mut();
            self.transfer_shape_layout(node_mut.connectable().shape(), &mut obj);
        }

        *out = Value::Object(obj);
    }

    fn transform_port(&mut self, port: &ElkPortRef, array: &mut Vec<Value>) {
        let mut obj = Map::new();
        let id = self.create_and_register_port(port);
        obj.insert("id".to_string(), Value::String(id));

        let labels = port_labels(port);
        if !labels.is_empty() {
            let mut labels_arr = Vec::new();
            for label in labels {
                self.transform_label(&label, &mut labels_arr);
            }
            obj.insert("labels".to_string(), Value::Array(labels_arr));
        }

        with_port_properties(port, |props| self.transform_properties(props, &mut obj));
        {
            let mut port_mut = port.borrow_mut();
            self.transfer_shape_layout(port_mut.connectable().shape(), &mut obj);
        }

        array.push(Value::Object(obj));
    }

    fn transform_edges(&mut self, node: &ElkNodeRef, root: &mut Value) {
        let edges = node_edges(node);
        if !edges.is_empty() {
            let pointer = self
                .node_ptr_map
                .get(&node_key(node))
                .map(|s| s.as_str())
                .unwrap_or("");
            if let Ok(json_node) = json_object_mut(root, pointer) {
                let mut edges_arr = Vec::new();
                for edge in edges {
                    self.transform_edge(&edge, &mut edges_arr);
                }
                json_node.insert("edges".to_string(), Value::Array(edges_arr));
            }
        }

        for child in node_children(node) {
            self.transform_edges(&child, root);
        }
    }

    fn transform_edge(&mut self, edge: &ElkEdgeRef, array: &mut Vec<Value>) {
        let mut obj = Map::new();
        let id = self.create_and_register_edge(edge);
        obj.insert("id".to_string(), Value::String(id));

        let mut sources = Vec::new();
        for source in edge.borrow().sources_ro().iter() {
            if let Some(id) = self.id_by_connectable(source) {
                sources.push(Value::String(id));
            }
        }
        obj.insert("sources".to_string(), Value::Array(sources));

        let mut targets = Vec::new();
        for target in edge.borrow().targets_ro().iter() {
            if let Some(id) = self.id_by_connectable(target) {
                targets.push(Value::String(id));
            }
        }
        obj.insert("targets".to_string(), Value::Array(targets));

        let labels = edge_labels(edge);
        if !labels.is_empty() {
            let mut labels_arr = Vec::new();
            for label in labels {
                self.transform_label(&label, &mut labels_arr);
            }
            obj.insert("labels".to_string(), Value::Array(labels_arr));
        }

        let sections = edge_sections(edge);
        if !self.omit_layout && !sections.is_empty() {
            let mut sections_arr = Vec::new();
            for section in sections {
                self.transform_section(&section, &mut sections_arr);
            }
            obj.insert("sections".to_string(), Value::Array(sections_arr));
        }

        if !self.omit_layout {
            let junction_points = with_edge_properties(edge, |props| {
                get_property_value(props, CoreOptions::JUNCTION_POINTS)
            });
            if let Some(jps) = junction_points {
                if !jps.is_empty() {
                    let mut json_jps = Vec::new();
                    for jp in jps.iter() {
                        json_jps.push(point_object(jp.x, jp.y));
                    }
                    obj.insert("junctionPoints".to_string(), Value::Array(json_jps));
                }
            }
        }

        with_edge_properties(edge, |props| self.transform_properties(props, &mut obj));
        array.push(Value::Object(obj));
    }

    fn transform_section(&mut self, section: &ElkEdgeSectionRef, sections_arr: &mut Vec<Value>) {
        let mut obj = Map::new();
        let id = self.create_and_register_edge_section(section);
        obj.insert("id".to_string(), Value::String(id));

        let start_point = {
            let section_ref = section.borrow();
            point_object(section_ref.start_x(), section_ref.start_y())
        };
        obj.insert("startPoint".to_string(), start_point);

        let end_point = {
            let section_ref = section.borrow();
            point_object(section_ref.end_x(), section_ref.end_y())
        };
        obj.insert("endPoint".to_string(), end_point);

        if !self.omit_layout {
            let bend_points = section.borrow_mut().bend_points().clone();
            if !bend_points.is_empty() {
                let mut bend_arr = Vec::new();
                for bend in bend_points {
                    let bend_ref = bend.borrow();
                    bend_arr.push(point_object(bend_ref.x(), bend_ref.y()));
                }
                obj.insert("bendPoints".to_string(), Value::Array(bend_arr));
            }
        }

        if let Some(shape) = section.borrow().incoming_shape() {
            if let Some(id) = self.id_by_connectable(&shape) {
                obj.insert("incomingShape".to_string(), Value::String(id));
            }
        }

        if let Some(shape) = section.borrow().outgoing_shape() {
            if let Some(id) = self.id_by_connectable(&shape) {
                obj.insert("outgoingShape".to_string(), Value::String(id));
            }
        }

        let incoming_sections = section.borrow().incoming_sections();
        if !incoming_sections.is_empty() {
            let mut incoming_arr = Vec::new();
            for sec in incoming_sections {
                if let Some(id) = self.id_by_section(&sec) {
                    incoming_arr.push(Value::String(id));
                }
            }
            obj.insert("incomingSections".to_string(), Value::Array(incoming_arr));
        }

        let outgoing_sections = section.borrow().outgoing_sections();
        if !outgoing_sections.is_empty() {
            let mut outgoing_arr = Vec::new();
            for sec in outgoing_sections {
                if let Some(id) = self.id_by_section(&sec) {
                    outgoing_arr.push(Value::String(id));
                }
            }
            obj.insert("outgoingSections".to_string(), Value::Array(outgoing_arr));
        }

        with_section_properties(section, |props| self.transform_properties(props, &mut obj));
        sections_arr.push(Value::Object(obj));
    }

    fn transform_label(&mut self, label: &ElkLabelRef, labels_arr: &mut Vec<Value>) {
        let mut obj = Map::new();
        let text = label.borrow().text().to_string();
        obj.insert("text".to_string(), Value::String(text));
        let identifier = {
            let mut label_mut = label.borrow_mut();
            label_mut
                .shape()
                .graph_element()
                .identifier()
                .map(|v| v.to_string())
        };
        if let Some(identifier) = identifier {
            if !identifier.is_empty() {
                obj.insert("id".to_string(), Value::String(identifier));
            }
        }

        with_label_properties(label, |props| self.transform_properties(props, &mut obj));
        {
            let mut label_mut = label.borrow_mut();
            self.transfer_shape_layout(label_mut.shape(), &mut obj);
        }

        labels_arr.push(Value::Object(obj));
    }

    fn transform_properties(&self, holder: &MapPropertyHolder, parent: &mut Map<String, Value>) {
        if holder.get_all_properties().is_empty() {
            return;
        }

        let mut json_props = Map::new();
        for (key, value) in holder.get_all_properties() {
            if key == CoreOptions::SPACING_INDIVIDUAL.id() {
                continue;
            }
            if self.omit_unknown_layout_options && !is_known_option(key.as_str()) {
                continue;
            }
            let out_key = if self.short_layout_option_keys {
                short_option_key(key.as_str())
            } else {
                key.to_string()
            };
            let value_str =
                property_value_to_string(key.as_str(), value).unwrap_or_else(|| "<value>".to_string());
            json_props.insert(out_key, Value::String(value_str));
        }

        if !json_props.is_empty() {
            parent.insert("layoutOptions".to_string(), Value::Object(json_props));
        }
    }

    fn transform_individual_spacings(
        &self,
        holder: &MapPropertyHolder,
        parent: &mut Map<String, Value>,
    ) {
        let Some(individual) = get_property_value(holder, CoreOptions::SPACING_INDIVIDUAL) else {
            return;
        };
        if individual.properties().get_all_properties().is_empty() {
            return;
        }
        let mut json_props = Map::new();
        for (key, value) in individual.properties().get_all_properties() {
            if self.omit_unknown_layout_options && !is_known_option(key.as_str()) {
                continue;
            }
            let out_key = if self.short_layout_option_keys {
                short_option_key(key.as_str())
            } else {
                key.to_string()
            };
            let value_str =
                property_value_to_string(key.as_str(), value).unwrap_or_else(|| "<value>".to_string());
            json_props.insert(out_key, Value::String(value_str));
        }
        if !json_props.is_empty() {
            parent.insert("individualSpacings".to_string(), Value::Object(json_props));
        }
    }

    fn transfer_shape_layout(
        &self,
        shape: &mut org_eclipse_elk_graph::org::eclipse::elk::graph::ElkShape,
        obj: &mut Map<String, Value>,
    ) {
        if !self.omit_layout {
            if !self.omit_zero_pos || shape.x() != 0.0 {
                obj.insert("x".to_string(), Value::Number(f64_to_number(shape.x())));
            }
            if !self.omit_zero_pos || shape.y() != 0.0 {
                obj.insert("y".to_string(), Value::Number(f64_to_number(shape.y())));
            }
        }

        if !self.omit_zero_dim || shape.width() != 0.0 {
            obj.insert(
                "width".to_string(),
                Value::Number(f64_to_number(shape.width())),
            );
        }
        if !self.omit_zero_dim || shape.height() != 0.0 {
            obj.insert(
                "height".to_string(),
                Value::Number(f64_to_number(shape.height())),
            );
        }
    }

    fn create_and_register_node(&mut self, node: &ElkNodeRef, pointer: &str) -> String {
        let mut id = node
            .borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .identifier()
            .map(|v| v.to_string())
            .unwrap_or_else(|| {
                let next = self.node_id_counter;
                self.node_id_counter += 1;
                format!("n{next}")
            });
        id = assert_unique(id, &mut self.node_ids_used, &mut self.unique_counter);
        self.node_id_map.insert(node_key(node), id.clone());
        self.node_ptr_map
            .insert(node_key(node), pointer.to_string());
        id
    }

    fn create_and_register_port(&mut self, port: &ElkPortRef) -> String {
        let mut id = port
            .borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .identifier()
            .map(|v| v.to_string())
            .unwrap_or_else(|| {
                let next = self.port_id_counter;
                self.port_id_counter += 1;
                format!("p{next}")
            });
        id = assert_unique(id, &mut self.port_ids_used, &mut self.unique_counter);
        self.port_id_map.insert(port_key(port), id.clone());
        id
    }

    fn create_and_register_edge(&mut self, edge: &ElkEdgeRef) -> String {
        let mut id = edge
            .borrow_mut()
            .element()
            .identifier()
            .map(|v| v.to_string())
            .unwrap_or_else(|| {
                let next = self.edge_id_counter;
                self.edge_id_counter += 1;
                format!("e{next}")
            });
        id = assert_unique(id, &mut self.edge_ids_used, &mut self.unique_counter);
        self.edge_id_map.insert(edge_key(edge), id.clone());
        id
    }

    fn create_and_register_edge_section(&mut self, section: &ElkEdgeSectionRef) -> String {
        let mut id = section
            .borrow()
            .identifier()
            .map(|v| v.to_string())
            .unwrap_or_else(|| {
                let next = self.edge_section_id_counter;
                self.edge_section_id_counter += 1;
                format!("s{next}")
            });
        id = assert_unique(
            id,
            &mut self.edge_section_ids_used,
            &mut self.unique_counter,
        );
        self.edge_section_id_map
            .insert(edge_section_key(section), id.clone());
        id
    }

    fn id_by_connectable(&self, shape: &ElkConnectableShapeRef) -> Option<String> {
        match shape {
            ElkConnectableShapeRef::Node(node) => self.node_id_map.get(&node_key(node)).cloned(),
            ElkConnectableShapeRef::Port(port) => self.port_id_map.get(&port_key(port)).cloned(),
        }
    }

    fn id_by_section(&self, section: &ElkEdgeSectionRef) -> Option<String> {
        self.edge_section_id_map
            .get(&edge_section_key(section))
            .cloned()
    }
}

fn assert_unique(id: String, used: &mut HashSet<String>, unique_counter: &mut u64) -> String {
    if !used.contains(&id) {
        used.insert(id.clone());
        return id;
    }
    let base = id;
    loop {
        let suffix = *unique_counter % 1_000_000;
        *unique_counter += 1;
        let candidate = format!("{base}_g{suffix:06}");
        if !used.contains(&candidate) {
            used.insert(candidate.clone());
            return candidate;
        }
    }
}

fn node_key(node: &ElkNodeRef) -> usize {
    std::rc::Rc::as_ptr(node) as usize
}

fn port_key(port: &ElkPortRef) -> usize {
    std::rc::Rc::as_ptr(port) as usize
}

fn edge_key(edge: &ElkEdgeRef) -> usize {
    std::rc::Rc::as_ptr(edge) as usize
}

fn edge_section_key(section: &ElkEdgeSectionRef) -> usize {
    std::rc::Rc::as_ptr(section) as usize
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

fn escape_pointer_segment(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn json_object_mut<'a>(
    root: &'a mut Value,
    pointer: &str,
) -> Result<&'a mut Map<String, Value>, ()> {
    let value = if pointer.is_empty() {
        root
    } else {
        root.pointer_mut(pointer).ok_or(())?
    };
    value.as_object_mut().ok_or(())
}

fn point_object(x: f64, y: f64) -> Value {
    let mut obj = Map::new();
    obj.insert("x".to_string(), Value::Number(f64_to_number(x)));
    obj.insert("y".to_string(), Value::Number(f64_to_number(y)));
    Value::Object(obj)
}

fn f64_to_number(value: f64) -> serde_json::Number {
    serde_json::Number::from_f64(value).unwrap_or_else(|| serde_json::Number::from(0))
}

fn node_children(node: &ElkNodeRef) -> Vec<ElkNodeRef> {
    node.borrow_mut().children().iter().cloned().collect()
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

fn edge_sections(edge: &ElkEdgeRef) -> Vec<ElkEdgeSectionRef> {
    let mut edge_mut = edge.borrow_mut();
    let list = edge_mut.sections();
    (0..list.len()).filter_map(|i| list.get(i)).collect()
}

fn with_node_properties<R>(node: &ElkNodeRef, f: impl FnOnce(&MapPropertyHolder) -> R) -> R {
    let mut node_mut = node.borrow_mut();
    let props = node_mut.connectable().shape().graph_element().properties();
    f(props)
}

fn with_port_properties<R>(port: &ElkPortRef, f: impl FnOnce(&MapPropertyHolder) -> R) -> R {
    let mut port_mut = port.borrow_mut();
    let props = port_mut.connectable().shape().graph_element().properties();
    f(props)
}

fn with_edge_properties<R>(edge: &ElkEdgeRef, f: impl FnOnce(&MapPropertyHolder) -> R) -> R {
    let mut edge_mut = edge.borrow_mut();
    let props = edge_mut.element().properties();
    f(props)
}

fn with_section_properties<R>(
    section: &ElkEdgeSectionRef,
    f: impl FnOnce(&MapPropertyHolder) -> R,
) -> R {
    let section_ref = section.borrow();
    f(section_ref.properties())
}

fn with_label_properties<R>(label: &ElkLabelRef, f: impl FnOnce(&MapPropertyHolder) -> R) -> R {
    let mut label_mut = label.borrow_mut();
    let props = label_mut.shape().graph_element().properties();
    f(props)
}

fn get_property_value<T: Clone + Send + Sync + 'static>(
    holder: &MapPropertyHolder,
    property: &Property<T>,
) -> Option<T> {
    holder
        .get_all_properties()
        .get(property.id())
        .and_then(|value| match value {
            PropertyValue::Resolved(value) => value.downcast_ref::<T>().cloned(),
            PropertyValue::Proxy(proxy) => proxy
                .resolve_value(property.id())
                .and_then(|resolved| resolved.downcast_ref::<T>().cloned()),
        })
}

fn property_value_to_string(property_id: &str, value: &PropertyValue) -> Option<String> {
    match value {
        PropertyValue::Resolved(value) => any_value_to_string(value),
        PropertyValue::Proxy(proxy) => proxy
            .resolve_value(property_id)
            .and_then(|resolved| any_value_to_string(&resolved)),
    }
}

fn any_value_to_string(value: &std::sync::Arc<dyn std::any::Any + Send + Sync>) -> Option<String> {
    if let Some(value) = value.downcast_ref::<f64>() {
        return Some(value.to_string());
    }
    if let Some(value) = value.downcast_ref::<i32>() {
        return Some(value.to_string());
    }
    if let Some(value) = value.downcast_ref::<bool>() {
        return Some(value.to_string());
    }
    if let Some(value) = value.downcast_ref::<String>() {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<KVector>() {
        return Some(value.to_string());
    }
    if let Some(value) = value.downcast_ref::<KVectorChain>() {
        return Some(value.to_string());
    }
    if let Some(value) = value.downcast_ref::<ElkMargin>() {
        return Some(spacing_to_string(
            value.top,
            value.left,
            value.bottom,
            value.right,
        ));
    }
    if let Some(value) = value.downcast_ref::<ElkPadding>() {
        return Some(spacing_to_string(
            value.top,
            value.left,
            value.bottom,
            value.right,
        ));
    }
    if let Some(value) = value.downcast_ref::<Alignment>() {
        return Some(enum_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<Direction>() {
        return Some(enum_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<EdgeRouting>() {
        return Some(enum_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<HierarchyHandling>() {
        return Some(enum_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<ShapeCoords>() {
        return Some(enum_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<EdgeCoords>() {
        return Some(enum_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<PortAlignment>() {
        return Some(enum_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<PortConstraints>() {
        return Some(enum_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<EdgeLabelPlacement>() {
        return Some(enum_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<EdgeType>() {
        return Some(enum_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<TopdownNodeTypes>() {
        return Some(enum_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<PortSide>() {
        return Some(enum_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<LabelSide>() {
        return Some(enum_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<EnumSet<ContentAlignment>>() {
        return Some(enumset_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<EnumSet<SizeConstraint>>() {
        return Some(enumset_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<EnumSet<SizeOptions>>() {
        return Some(enumset_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<EnumSet<NodeLabelPlacement>>() {
        return Some(enumset_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<EnumSet<PortLabelPlacement>>() {
        return Some(enumset_to_string(value));
    }
    if let Some(value) = value.downcast_ref::<IndividualSpacings>() {
        return Some(value.to_string());
    }
    Some("<value>".to_string())
}

fn spacing_to_string(top: f64, left: f64, bottom: f64, right: f64) -> String {
    format!("[top={top},left={left},bottom={bottom},right={right}]")
}

fn enum_to_string<T: std::fmt::Debug>(value: &T) -> String {
    to_upper_snake(&format!("{:?}", value))
}

fn enumset_to_string<T: EnumSetType + std::fmt::Debug>(value: &EnumSet<T>) -> String {
    let values: Vec<String> = value.iter().map(enum_to_string).collect();
    format!("[{}]", values.join(", "))
}

fn to_upper_snake(value: &str) -> String {
    let mut out = String::new();
    let mut prev: Option<char> = None;
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        let next = chars.peek().copied();
        if let Some(prev_ch) = prev {
            if ch.is_uppercase()
                && (prev_ch.is_lowercase() || next.map(|n| n.is_lowercase()).unwrap_or(false))
            {
                out.push('_');
            }
        }
        out.push(ch.to_ascii_uppercase());
        prev = Some(ch);
    }
    out
}

fn is_known_option(id: &str) -> bool {
    LayoutMetaDataService::get_instance()
        .get_option_data_by_suffix(id)
        .is_some()
}

fn short_option_key(full_id: &str) -> String {
    full_id
        .strip_prefix("org.eclipse.")
        .unwrap_or(full_id)
        .to_string()
}
