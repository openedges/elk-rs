use std::collections::HashMap;
use std::panic::panic_any;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LGraphRef, LGraphUtil, LNodeRef};
use crate::org::eclipse::elk::alg::layered::graph::l_node::NodeType;
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

/// Container for spacing values configured in core or layered options.
#[derive(Clone)]
pub struct Spacings {
    graph: LGraphRef,
    node_type_spacing_options_horizontal: Vec<Vec<Option<&'static Property<f64>>>>,
    node_type_spacing_options_vertical: Vec<Vec<Option<&'static Property<f64>>>>,
    graph_property_values: HashMap<usize, f64>,
}

impl Spacings {
    pub fn new(graph: &LGraphRef) -> Self {
        let mut spacings = Spacings {
            graph: graph.clone(),
            node_type_spacing_options_horizontal: vec![vec![None; NodeType::COUNT]; NodeType::COUNT],
            node_type_spacing_options_vertical: vec![vec![None; NodeType::COUNT]; NodeType::COUNT],
            graph_property_values: HashMap::new(),
        };
        spacings.precalculate_node_type_spacings();
        spacings.cache_graph_values();
        spacings
    }

    fn precalculate_node_type_spacings(&mut self) {
        // normal
        self.node_type_spacing_same(
            NodeType::Normal,
            LayeredOptions::SPACING_NODE_NODE,
            LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS,
        );
        self.node_type_spacing(
            NodeType::Normal,
            NodeType::LongEdge,
            LayeredOptions::SPACING_EDGE_NODE,
            LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS,
        );
        self.node_type_spacing_single(
            NodeType::Normal,
            NodeType::NorthSouthPort,
            LayeredOptions::SPACING_EDGE_NODE,
        );
        self.node_type_spacing_single(
            NodeType::Normal,
            NodeType::ExternalPort,
            LayeredOptions::SPACING_EDGE_NODE,
        );
        self.node_type_spacing(
            NodeType::Normal,
            NodeType::Label,
            LayeredOptions::SPACING_NODE_NODE,
            LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS,
        );

        // longedge
        self.node_type_spacing_same(
            NodeType::LongEdge,
            LayeredOptions::SPACING_EDGE_EDGE,
            LayeredOptions::SPACING_EDGE_EDGE_BETWEEN_LAYERS,
        );
        self.node_type_spacing_single(
            NodeType::LongEdge,
            NodeType::NorthSouthPort,
            LayeredOptions::SPACING_EDGE_EDGE,
        );
        self.node_type_spacing_single(
            NodeType::LongEdge,
            NodeType::ExternalPort,
            LayeredOptions::SPACING_EDGE_EDGE,
        );
        self.node_type_spacing(
            NodeType::LongEdge,
            NodeType::Label,
            LayeredOptions::SPACING_EDGE_NODE,
            LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS,
        );

        // northsouth
        self.node_type_spacing_self(NodeType::NorthSouthPort, LayeredOptions::SPACING_EDGE_EDGE);
        self.node_type_spacing_single(
            NodeType::NorthSouthPort,
            NodeType::ExternalPort,
            LayeredOptions::SPACING_EDGE_EDGE,
        );
        self.node_type_spacing_single(
            NodeType::NorthSouthPort,
            NodeType::Label,
            LayeredOptions::SPACING_LABEL_NODE,
        );

        // external
        self.node_type_spacing_self(NodeType::ExternalPort, LayeredOptions::SPACING_PORT_PORT);
        self.node_type_spacing(
            NodeType::ExternalPort,
            NodeType::Label,
            LayeredOptions::SPACING_LABEL_PORT_VERTICAL,
            LayeredOptions::SPACING_LABEL_PORT_HORIZONTAL,
        );

        // label
        self.node_type_spacing_same(
            NodeType::Label,
            LayeredOptions::SPACING_EDGE_EDGE,
            LayeredOptions::SPACING_EDGE_EDGE,
        );

        // breaking points
        self.node_type_spacing_same(
            NodeType::BreakingPoint,
            LayeredOptions::SPACING_EDGE_EDGE,
            LayeredOptions::SPACING_EDGE_EDGE_BETWEEN_LAYERS,
        );
        self.node_type_spacing(
            NodeType::BreakingPoint,
            NodeType::Normal,
            LayeredOptions::SPACING_EDGE_NODE,
            LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS,
        );
        self.node_type_spacing(
            NodeType::BreakingPoint,
            NodeType::Label,
            LayeredOptions::SPACING_EDGE_NODE,
            LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS,
        );
        self.node_type_spacing(
            NodeType::BreakingPoint,
            NodeType::LongEdge,
            LayeredOptions::SPACING_EDGE_NODE,
            LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS,
        );
    }

    fn node_type_spacing_self(&mut self, nt: NodeType, spacing: &'static Property<f64>) {
        let index = nt.ordinal();
        self.node_type_spacing_options_vertical[index][index] = Some(spacing);
    }

    fn node_type_spacing_same(
        &mut self,
        nt: NodeType,
        spacing_vert: &'static Property<f64>,
        spacing_horz: &'static Property<f64>,
    ) {
        let index = nt.ordinal();
        self.node_type_spacing_options_vertical[index][index] = Some(spacing_vert);
        self.node_type_spacing_options_horizontal[index][index] = Some(spacing_horz);
    }

    fn node_type_spacing_single(
        &mut self,
        n1: NodeType,
        n2: NodeType,
        spacing: &'static Property<f64>,
    ) {
        let idx1 = n1.ordinal();
        let idx2 = n2.ordinal();
        self.node_type_spacing_options_vertical[idx1][idx2] = Some(spacing);
        self.node_type_spacing_options_vertical[idx2][idx1] = Some(spacing);
    }

    fn node_type_spacing(
        &mut self,
        n1: NodeType,
        n2: NodeType,
        spacing_vert: &'static Property<f64>,
        spacing_horz: &'static Property<f64>,
    ) {
        let idx1 = n1.ordinal();
        let idx2 = n2.ordinal();
        self.node_type_spacing_options_vertical[idx1][idx2] = Some(spacing_vert);
        self.node_type_spacing_options_vertical[idx2][idx1] = Some(spacing_vert);
        self.node_type_spacing_options_horizontal[idx1][idx2] = Some(spacing_horz);
        self.node_type_spacing_options_horizontal[idx2][idx1] = Some(spacing_horz);
    }

    pub fn get_horizontal_spacing(&self, n1: &LNodeRef, n2: &LNodeRef) -> f64 {
        self.get_local_spacing(n1, n2, &self.node_type_spacing_options_horizontal)
    }

    pub fn get_vertical_spacing(&self, n1: &LNodeRef, n2: &LNodeRef) -> f64 {
        self.get_local_spacing(n1, n2, &self.node_type_spacing_options_vertical)
    }

    pub fn get_horizontal_spacing_for_types(&self, nt1: NodeType, nt2: NodeType) -> f64 {
        self.get_local_spacing_for_types(nt1, nt2, &self.node_type_spacing_options_horizontal)
    }

    pub fn get_vertical_spacing_for_types(&self, nt1: NodeType, nt2: NodeType) -> f64 {
        self.get_local_spacing_for_types(nt1, nt2, &self.node_type_spacing_options_vertical)
    }

    fn get_local_spacing(
        &self,
        n1: &LNodeRef,
        n2: &LNodeRef,
        mapping: &[Vec<Option<&'static Property<f64>>>],
    ) -> f64 {
        let t1 = n1
            .lock()
            .map(|node| node.node_type())
            .unwrap_or(NodeType::Normal);
        let t2 = n2
            .lock()
            .map(|node| node.node_type())
            .unwrap_or(NodeType::Normal);
        let layout_option = mapping[t1.ordinal()][t2.ordinal()]
            .unwrap_or_else(|| panic_any(UnspecifiedSpacingException::new(None)));
        let s1 = self.get_individual_or_default_f64(n1, layout_option);
        let s2 = self.get_individual_or_default_f64(n2, layout_option);
        s1.max(s2)
    }

    fn get_local_spacing_for_types(
        &self,
        nt1: NodeType,
        nt2: NodeType,
        mapping: &[Vec<Option<&'static Property<f64>>>],
    ) -> f64 {
        let layout_option = mapping[nt1.ordinal()][nt2.ordinal()]
            .unwrap_or_else(|| panic_any(UnspecifiedSpacingException::new(None)));
        self.graph_spacing_value(layout_option)
    }

    fn cache_graph_values(&mut self) {
        let mut values: HashMap<usize, f64> = HashMap::new();
        let mut graph_guard = self.graph.lock().ok();

        let mut cache = |property: &'static Property<f64>| {
            let key = property as *const Property<f64> as usize;
            if values.contains_key(&key) {
                return;
            }
            let value = if let Some(graph_guard) = graph_guard.as_mut() {
                graph_guard
                    .get_property(property)
                    .or_else(|| property.get_default())
                    .unwrap_or_else(|| panic_any(UnspecifiedSpacingException::new(None)))
            } else {
                property
                    .get_default()
                    .unwrap_or_else(|| panic_any(UnspecifiedSpacingException::new(None)))
            };
            values.insert(key, value);
        };

        for row in &self.node_type_spacing_options_vertical {
            for property in row.iter().flatten() {
                cache(property);
            }
        }
        for row in &self.node_type_spacing_options_horizontal {
            for property in row.iter().flatten() {
                cache(property);
            }
        }

        self.graph_property_values = values;
    }

    fn graph_spacing_value(&self, property: &'static Property<f64>) -> f64 {
        let key = property as *const Property<f64> as usize;
        if let Some(value) = self.graph_property_values.get(&key) {
            return *value;
        }
        property
            .get_default()
            .unwrap_or_else(|| panic_any(UnspecifiedSpacingException::new(None)))
    }

    fn get_individual_or_default_f64(
        &self,
        node: &LNodeRef,
        property: &'static Property<f64>,
    ) -> f64 {
        let mut value = None;
        if let Ok(mut node_guard) = node.lock() {
            let has_individual = node_guard
                .shape()
                .graph_element()
                .properties()
                .has_property(CoreOptions::SPACING_INDIVIDUAL);
            if has_individual {
                if let Some(mut individual) =
                    node_guard.get_property(CoreOptions::SPACING_INDIVIDUAL)
                {
                    let has_prop = individual.properties().has_property(property);
                    if has_prop {
                        value = individual.properties_mut().get_property(property);
                    }
                }
            }
        }
        if let Some(value) = value {
            return value;
        }
        self.graph_spacing_value(property)
    }

    pub fn get_individual_or_default<T: Clone + Send + Sync + 'static>(
        node: &LNodeRef,
        property: &Property<T>,
    ) -> T {
        if let Some(value) = LGraphUtil::get_individual_or_inherited(node, property) {
            return value;
        }
        property
            .get_default()
            .unwrap_or_else(|| panic_any(UnspecifiedSpacingException::new(None)))
    }

    pub fn get_individual_or_default_with_graph<T: Clone + Send + Sync + 'static>(
        graph: &LGraph,
        node: &LNodeRef,
        property: &Property<T>,
    ) -> T {
        let mut value = None;
        if let Ok(mut node_guard) = node.lock() {
            let has_individual = node_guard
                .shape()
                .graph_element()
                .properties()
                .has_property(CoreOptions::SPACING_INDIVIDUAL);
            if has_individual {
                if let Some(mut individual) =
                    node_guard.get_property(CoreOptions::SPACING_INDIVIDUAL)
                {
                    let has_prop = individual.properties().has_property(property);
                    if has_prop {
                        value = individual.properties_mut().get_property(property);
                    }
                }
            }
        }
        if let Some(value) = value {
            return value;
        }
        if let Some(value) = graph.get_property_ref(property) {
            return value;
        }
        property
            .get_default()
            .unwrap_or_else(|| panic_any(UnspecifiedSpacingException::new(None)))
    }
}

#[derive(Clone, Debug)]
pub struct UnspecifiedSpacingException {
    message: Option<String>,
}

impl UnspecifiedSpacingException {
    pub fn new(message: Option<String>) -> Self {
        UnspecifiedSpacingException { message }
    }
}

impl std::fmt::Display for UnspecifiedSpacingException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(message) = &self.message {
            write!(f, "{message}")
        } else {
            write!(f, "Unspecified spacing")
        }
    }
}

impl std::error::Error for UnspecifiedSpacingException {}
