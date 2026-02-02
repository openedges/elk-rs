use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::org::eclipse::elk::graph::{
    ElkEdgeRef, ElkEdgeSectionRef, ElkGraphElementRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

pub struct GraphIdentifierGenerator {
    graph: ElkNodeRef,
    validate: bool,
    generate: bool,
    unique: bool,
    current_ids: [u32; 5],
    existing_identifiers: HashSet<String>,
    rng_state: Option<u64>,
}

impl GraphIdentifierGenerator {
    pub fn for_graph(graph: ElkNodeRef) -> Self {
        let mut generator = GraphIdentifierGenerator {
            graph,
            validate: false,
            generate: false,
            unique: false,
            current_ids: [0; 5],
            existing_identifiers: HashSet::new(),
            rng_state: None,
        };
        generator.collect_existing_identifiers();
        generator
    }

    pub fn assert_valid(&mut self) -> &mut Self {
        self.validate = true;
        self
    }

    pub fn assert_exists(&mut self) -> &mut Self {
        self.generate = true;
        self
    }

    pub fn assert_unique(&mut self) -> &mut Self {
        self.unique = true;
        self
    }

    pub fn execute(&mut self) {
        if self.validate {
            self.validate_identifiers(&self.graph);
        }
        if self.generate {
            self.generate_identifiers(&self.graph);
        }
        if self.unique {
            self.assert_all_ids_unique(&self.graph);
        }
    }

    fn collect_existing_identifiers(&mut self) {
        let mut elements = Vec::new();
        collect_graph_elements(&self.graph, false, &mut elements);
        for element in elements {
            if let Some(identifier) = element_identifier(&element) {
                if !identifier.trim().is_empty() {
                    self.existing_identifiers.insert(identifier);
                }
            }
        }

        let mut sections = Vec::new();
        collect_edge_sections(&self.graph, &mut sections);
        for section in sections {
            if let Some(identifier) = edge_section_identifier(&section) {
                if !identifier.trim().is_empty() {
                    self.existing_identifiers.insert(identifier);
                }
            }
        }
    }

    fn validate_identifiers(&mut self, node: &ElkNodeRef) {
        self.validate_graph_element(&ElkGraphElementRef::Node(node.clone()));

        let contents = node_contents(node);
        for label in contents.labels {
            self.validate_label(&label);
        }
        for port in contents.ports {
            self.validate_port(&port);
        }
        for edge in contents.edges {
            self.validate_edge(&edge);
        }
        for child in contents.children {
            self.validate_identifiers(&child);
        }
    }

    fn validate_port(&mut self, port: &ElkPortRef) {
        self.validate_graph_element(&ElkGraphElementRef::Port(port.clone()));
        for label in port_labels(port) {
            self.validate_label(&label);
        }
    }

    fn validate_label(&mut self, label: &ElkLabelRef) {
        self.validate_graph_element(&ElkGraphElementRef::Label(label.clone()));
        for child in label_labels(label) {
            self.validate_label(&child);
        }
    }

    fn validate_edge(&mut self, edge: &ElkEdgeRef) {
        self.validate_graph_element(&ElkGraphElementRef::Edge(edge.clone()));
        let contents = edge_contents(edge);
        for label in contents.labels {
            self.validate_label(&label);
        }
        for section in contents.sections {
            self.validate_edge_section(&section);
        }
    }

    fn validate_graph_element(&mut self, element: &ElkGraphElementRef) {
        let updated = validate_identifier(element_identifier(element).as_deref());
        if let Some(identifier) = updated {
            set_element_identifier(element, Some(identifier));
        }
    }

    fn validate_edge_section(&mut self, section: &ElkEdgeSectionRef) {
        let updated = validate_identifier(edge_section_identifier(section).as_deref());
        if let Some(identifier) = updated {
            set_edge_section_identifier(section, Some(identifier));
        }
    }

    fn generate_identifiers(&mut self, node: &ElkNodeRef) {
        let is_root = node.borrow().parent().is_none();
        if is_root {
            let identifier = node_identifier(node);
            if is_missing_identifier(identifier.as_deref()) {
                set_node_identifier(node, Some("G1".to_string()));
            }
        } else {
            self.set_identifier_if_missing(&ElkGraphElementRef::Node(node.clone()), ElementType::Node);
        }

        let contents = node_contents(node);
        for label in contents.labels {
            self.generate_label(&label);
        }
        for port in contents.ports {
            self.generate_port(&port);
        }
        for edge in contents.edges {
            self.generate_edge(&edge);
        }
        for child in contents.children {
            self.generate_identifiers(&child);
        }
    }

    fn generate_port(&mut self, port: &ElkPortRef) {
        self.set_identifier_if_missing(&ElkGraphElementRef::Port(port.clone()), ElementType::Port);
        for label in port_labels(port) {
            self.generate_label(&label);
        }
    }

    fn generate_label(&mut self, label: &ElkLabelRef) {
        self.set_identifier_if_missing(&ElkGraphElementRef::Label(label.clone()), ElementType::Label);
        for child in label_labels(label) {
            self.generate_label(&child);
        }
    }

    fn generate_edge(&mut self, edge: &ElkEdgeRef) {
        self.set_identifier_if_missing(&ElkGraphElementRef::Edge(edge.clone()), ElementType::Edge);
        let contents = edge_contents(edge);
        for label in contents.labels {
            self.generate_label(&label);
        }
        for section in contents.sections {
            self.set_edge_section_identifier_if_missing(&section);
        }
    }

    fn set_identifier_if_missing(&mut self, element: &ElkGraphElementRef, element_type: ElementType) {
        let identifier = element_identifier(element);
        if is_missing_identifier(identifier.as_deref()) {
            let new_identifier = self.next_identifier(element_type);
            set_element_identifier(element, Some(new_identifier));
        }
    }

    fn set_edge_section_identifier_if_missing(&mut self, section: &ElkEdgeSectionRef) {
        let identifier = edge_section_identifier(section);
        if is_missing_identifier(identifier.as_deref()) {
            let new_identifier = self.next_identifier(ElementType::EdgeSection);
            set_edge_section_identifier(section, Some(new_identifier));
        }
    }

    fn next_identifier(&mut self, element_type: ElementType) -> String {
        let index = element_type.index();
        let prefix = element_type.prefix();
        loop {
            self.current_ids[index] += 1;
            let identifier = format!("{}{}", prefix, self.current_ids[index]);
            if !self.existing_identifiers.contains(&identifier) {
                return identifier;
            }
        }
    }

    fn assert_all_ids_unique(&mut self, node: &ElkNodeRef) {
        let mut elements = Vec::new();
        collect_graph_elements(node, false, &mut elements);

        let mut known_ids = HashSet::new();
        let mut seen_null = false;
        for element in elements {
            let identifier = element_identifier(&element);
            match identifier {
                None => {
                    if seen_null {
                        let mut new_id =
                            format!("null_g{}", self.four_digit_padded_random_number());
                        while known_ids.contains(&new_id) {
                            new_id =
                                format!("{}_g{}", new_id, self.four_digit_padded_random_number());
                        }
                        set_element_identifier(&element, Some(new_id.clone()));
                        known_ids.insert(new_id);
                    } else {
                        seen_null = true;
                    }
                }
                Some(mut identifier) => {
                    while known_ids.contains(&identifier) {
                        identifier = format!("{}_g{}", identifier, self.four_digit_padded_random_number());
                    }
                    if element_identifier(&element).as_deref() != Some(identifier.as_str()) {
                        set_element_identifier(&element, Some(identifier.clone()));
                    }
                    known_ids.insert(identifier);
                }
            }
        }
    }

    fn four_digit_padded_random_number(&mut self) -> String {
        format!("{:04}", self.next_random_u32() % 10_000)
    }

    fn next_random_u32(&mut self) -> u32 {
        let state = match self.rng_state {
            Some(state) => state,
            None => {
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;
                let seed = seed ^ (seed >> 12);
                self.rng_state = Some(seed);
                seed
            }
        };
        let next = state
            .wrapping_mul(6364136223846793005_u64)
            .wrapping_add(1);
        self.rng_state = Some(next);
        (next >> 32) as u32
    }
}

#[derive(Clone, Copy, Debug)]
enum ElementType {
    Node,
    Port,
    Edge,
    EdgeSection,
    Label,
}

impl ElementType {
    fn prefix(self) -> &'static str {
        match self {
            ElementType::Node => "N",
            ElementType::Port => "P",
            ElementType::Edge => "E",
            ElementType::EdgeSection => "ES",
            ElementType::Label => "L",
        }
    }

    fn index(self) -> usize {
        match self {
            ElementType::Node => 0,
            ElementType::Port => 1,
            ElementType::Edge => 2,
            ElementType::EdgeSection => 3,
            ElementType::Label => 4,
        }
    }
}

struct NodeContents {
    labels: Vec<ElkLabelRef>,
    ports: Vec<ElkPortRef>,
    edges: Vec<ElkEdgeRef>,
    children: Vec<ElkNodeRef>,
}

fn node_contents(node: &ElkNodeRef) -> NodeContents {
    let mut node_mut = node.borrow_mut();
    let labels = node_mut
        .connectable()
        .shape()
        .graph_element()
        .labels()
        .iter()
        .cloned()
        .collect();
    let ports = node_mut.ports().iter().cloned().collect();
    let edges = node_mut.contained_edges().iter().cloned().collect();
    let children = node_mut.children().iter().cloned().collect();
    NodeContents {
        labels,
        ports,
        edges,
        children,
    }
}

struct EdgeContents {
    labels: Vec<ElkLabelRef>,
    sections: Vec<ElkEdgeSectionRef>,
}

fn edge_contents(edge: &ElkEdgeRef) -> EdgeContents {
    let mut edge_mut = edge.borrow_mut();
    let labels = edge_mut.element().labels().iter().cloned().collect();
    let sections = edge_mut.sections().iter().cloned().collect();
    EdgeContents { labels, sections }
}

fn port_labels(port: &ElkPortRef) -> Vec<ElkLabelRef> {
    let mut port_mut = port.borrow_mut();
    port_mut
        .connectable()
        .shape()
        .graph_element()
        .labels()
        .iter()
        .cloned()
        .collect()
}

fn label_labels(label: &ElkLabelRef) -> Vec<ElkLabelRef> {
    let mut label_mut = label.borrow_mut();
    label_mut
        .shape
        .graph_element()
        .labels()
        .iter()
        .cloned()
        .collect()
}

fn element_identifier(element: &ElkGraphElementRef) -> Option<String> {
    match element {
        ElkGraphElementRef::Node(node) => node_identifier(node),
        ElkGraphElementRef::Port(port) => port_identifier(port),
        ElkGraphElementRef::Edge(edge) => edge_identifier(edge),
        ElkGraphElementRef::Label(label) => label_identifier(label),
    }
}

fn set_element_identifier(element: &ElkGraphElementRef, identifier: Option<String>) {
    match element {
        ElkGraphElementRef::Node(node) => set_node_identifier(node, identifier),
        ElkGraphElementRef::Port(port) => set_port_identifier(port, identifier),
        ElkGraphElementRef::Edge(edge) => set_edge_identifier(edge, identifier),
        ElkGraphElementRef::Label(label) => set_label_identifier(label, identifier),
    }
}

fn node_identifier(node: &ElkNodeRef) -> Option<String> {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .identifier()
        .map(|id| id.to_string())
}

fn set_node_identifier(node: &ElkNodeRef, identifier: Option<String>) {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .set_identifier(identifier);
}

fn port_identifier(port: &ElkPortRef) -> Option<String> {
    let mut port_mut = port.borrow_mut();
    port_mut
        .connectable()
        .shape()
        .graph_element()
        .identifier()
        .map(|id| id.to_string())
}

fn set_port_identifier(port: &ElkPortRef, identifier: Option<String>) {
    let mut port_mut = port.borrow_mut();
    port_mut
        .connectable()
        .shape()
        .graph_element()
        .set_identifier(identifier);
}

fn edge_identifier(edge: &ElkEdgeRef) -> Option<String> {
    let mut edge_mut = edge.borrow_mut();
    edge_mut.element().identifier().map(|id| id.to_string())
}

fn set_edge_identifier(edge: &ElkEdgeRef, identifier: Option<String>) {
    let mut edge_mut = edge.borrow_mut();
    edge_mut.element().set_identifier(identifier);
}

fn label_identifier(label: &ElkLabelRef) -> Option<String> {
    let mut label_mut = label.borrow_mut();
    label_mut
        .shape
        .graph_element()
        .identifier()
        .map(|id| id.to_string())
}

fn set_label_identifier(label: &ElkLabelRef, identifier: Option<String>) {
    let mut label_mut = label.borrow_mut();
    label_mut.shape.graph_element().set_identifier(identifier);
}

fn edge_section_identifier(section: &ElkEdgeSectionRef) -> Option<String> {
    section.borrow().identifier().map(|id| id.to_string())
}

fn set_edge_section_identifier(section: &ElkEdgeSectionRef, identifier: Option<String>) {
    section.borrow_mut().set_identifier(identifier);
}

fn validate_identifier(identifier: Option<&str>) -> Option<String> {
    let identifier = identifier?;
    if identifier.is_empty() {
        return None;
    }

    let mut valid = true;
    let mut chars: Vec<char> = Vec::with_capacity(identifier.len());
    for (index, ch) in identifier.chars().enumerate() {
        let allowed = ch.is_ascii_alphabetic()
            || ch == '_'
            || (index > 0 && ch.is_ascii_digit());
        if allowed {
            chars.push(ch);
        } else {
            chars.push('_');
            valid = false;
        }
    }

    if valid {
        None
    } else {
        Some(chars.into_iter().collect())
    }
}

fn is_missing_identifier(identifier: Option<&str>) -> bool {
    identifier.map(|value| value.trim().is_empty()).unwrap_or(true)
}

fn collect_graph_elements(
    node: &ElkNodeRef,
    include_self: bool,
    out: &mut Vec<ElkGraphElementRef>,
) {
    if include_self {
        out.push(ElkGraphElementRef::Node(node.clone()));
    }

    let contents = node_contents(node);
    for label in contents.labels {
        collect_label_elements(&label, out);
    }
    for port in contents.ports {
        collect_port_elements(&port, out);
    }
    for edge in contents.edges {
        collect_edge_elements(&edge, out);
    }
    for child in contents.children {
        collect_graph_elements(&child, true, out);
    }
}

fn collect_port_elements(port: &ElkPortRef, out: &mut Vec<ElkGraphElementRef>) {
    out.push(ElkGraphElementRef::Port(port.clone()));
    for label in port_labels(port) {
        collect_label_elements(&label, out);
    }
}

fn collect_label_elements(label: &ElkLabelRef, out: &mut Vec<ElkGraphElementRef>) {
    out.push(ElkGraphElementRef::Label(label.clone()));
    for child in label_labels(label) {
        collect_label_elements(&child, out);
    }
}

fn collect_edge_elements(edge: &ElkEdgeRef, out: &mut Vec<ElkGraphElementRef>) {
    out.push(ElkGraphElementRef::Edge(edge.clone()));
    let contents = edge_contents(edge);
    for label in contents.labels {
        collect_label_elements(&label, out);
    }
}

fn collect_edge_sections(node: &ElkNodeRef, out: &mut Vec<ElkEdgeSectionRef>) {
    let contents = node_contents(node);
    for edge in contents.edges {
        out.extend(edge_contents(&edge).sections);
    }
    for child in contents.children {
        collect_edge_sections(&child, out);
    }
}
