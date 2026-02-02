use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef;

use crate::org::eclipse::elk::core::options::CoreOptions;
use crate::org::eclipse::elk::core::util::IGraphElementVisitor;

pub struct ElkSpacings;

impl ElkSpacings {
    pub fn with_base_value(base_spacing: f64) -> ElkCoreSpacingsBuilder {
        ElkCoreSpacingsBuilder::new(base_spacing)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SpacingFactor {
    pub property: &'static Property<f64>,
    pub factor: f64,
}

#[derive(Clone)]
pub struct SpacingConfigurator {
    base_spacing: f64,
    overwrite: bool,
    factors: Vec<SpacingFactor>,
    no_op: bool,
}

impl SpacingConfigurator {
    pub fn apply_to_properties(&self, props: &mut MapPropertyHolder) {
        if self.no_op {
            return;
        }
        for entry in &self.factors {
            if !self.overwrite && props.has_property(entry.property) {
                continue;
            }
            let value = entry.factor * self.base_spacing;
            props.set_property(entry.property, Some(value));
        }
    }

    pub fn apply(&self, element: &ElkGraphElementRef) {
        with_properties_mut(element, |props| self.apply_to_properties(props));
    }

    pub fn to_visitor(&self) -> Box<dyn IGraphElementVisitor> {
        Box::new(SpacingVisitor {
            configurator: self.clone(),
        })
    }
}

struct SpacingVisitor {
    configurator: SpacingConfigurator,
}

impl IGraphElementVisitor for SpacingVisitor {
    fn visit(&mut self, element: &ElkGraphElementRef) {
        self.configurator.apply(element);
    }
}

pub struct ElkCoreSpacingsBuilder {
    base_spacing: f64,
    overwrite: bool,
    factors: Vec<SpacingFactor>,
}

impl ElkCoreSpacingsBuilder {
    pub const BASE_SPACING_OPTION: &'static LazyLock<Property<f64>> = CoreOptions::SPACING_NODE_NODE;
    const DOUBLE_EQ_EPSILON: f64 = 10e-5;

    fn new(base_spacing: f64) -> Self {
        let base_option: &Property<f64> = Self::BASE_SPACING_OPTION;
        let base_default = base_option
            .get_default()
            .unwrap_or(0.0);
        if fuzzy_equals(base_default, 0.0, Self::DOUBLE_EQ_EPSILON) {
            panic!("Base spacing default value must be different from 0.0.");
        }

        let mut factors = Vec::with_capacity(1 + Self::dependent_spacing_options().len());
        factors.push(SpacingFactor {
            property: base_option,
            factor: 1.0,
        });

        for option in Self::dependent_spacing_options() {
            let option_ref: &Property<f64> = option;
            let factor = match (base_default, option_ref.get_default()) {
                (base, Some(default)) => default / base,
                _ => 1.0,
            };
            factors.push(SpacingFactor {
                property: option_ref,
                factor,
            });
        }

        ElkCoreSpacingsBuilder {
            base_spacing,
            overwrite: false,
            factors,
        }
    }

    fn dependent_spacing_options() -> &'static [&'static LazyLock<Property<f64>>] {
        static OPTIONS: [&LazyLock<Property<f64>>; 10] = [
            CoreOptions::SPACING_COMPONENT_COMPONENT,
            CoreOptions::SPACING_EDGE_EDGE,
            CoreOptions::SPACING_EDGE_LABEL,
            CoreOptions::SPACING_EDGE_NODE,
            CoreOptions::SPACING_LABEL_LABEL,
            CoreOptions::SPACING_LABEL_NODE,
            CoreOptions::SPACING_LABEL_PORT_HORIZONTAL,
            CoreOptions::SPACING_LABEL_PORT_VERTICAL,
            CoreOptions::SPACING_NODE_SELF_LOOP,
            CoreOptions::SPACING_PORT_PORT,
        ];
        &OPTIONS
    }

    fn compute_factor(&self, value: f64) -> f64 {
        if fuzzy_equals(self.base_spacing, 0.0, Self::DOUBLE_EQ_EPSILON) {
            panic!("Base spacing must not be 0.0.");
        }
        value / self.base_spacing
    }

    fn find_factor_index(&self, spacing_option: &Property<f64>) -> Option<usize> {
        let id = spacing_option.id();
        self.factors
            .iter()
            .position(|entry| entry.property.id() == id)
    }

    pub fn factors(&self) -> &[SpacingFactor] {
        &self.factors
    }

    pub fn with_factor(&mut self, spacing_option: &Property<f64>, factor: f64) -> &mut Self {
        if self.find_factor_index(spacing_option).is_none() {
            panic!(
                "'{}' is not a configurable spacing option.",
                spacing_option.id()
            );
        }
        if factor < 0.0 {
            panic!(
                "The factor for '{}' must not be negative ({}).",
                spacing_option.id(),
                factor
            );
        }
        if spacing_option.id() == Self::BASE_SPACING_OPTION.id() {
            panic!(
                "'{}' is the base spacing option not allowed to use with 'with_factor'.",
                spacing_option.id()
            );
        }
        let index = self.find_factor_index(spacing_option).unwrap();
        self.factors[index].factor = factor;
        self
    }

    pub fn with_value(&mut self, spacing_option: &Property<f64>, value: f64) -> &mut Self {
        if self.find_factor_index(spacing_option).is_none() {
            panic!(
                "'{}' is not a configurable spacing option.",
                spacing_option.id()
            );
        }
        if value < 0.0 {
            panic!(
                "The value for '{}' must not be negative ({}).",
                spacing_option.id(),
                value
            );
        }
        if spacing_option.id() == Self::BASE_SPACING_OPTION.id() {
            panic!(
                "'{}' is the base spacing option not allowed to use with 'with_value'.",
                spacing_option.id()
            );
        }
        let factor = self.compute_factor(value);
        let index = self.find_factor_index(spacing_option).unwrap();
        self.factors[index].factor = factor;
        self
    }

    pub fn with_overwrite(&mut self, overwrite: bool) -> &mut Self {
        self.overwrite = overwrite;
        self
    }

    pub fn build(&self) -> SpacingConfigurator {
        let base_default = Self::BASE_SPACING_OPTION
            .get_default()
            .unwrap_or(0.0);
        let no_op = !self.overwrite
            && fuzzy_equals(self.base_spacing, base_default, Self::DOUBLE_EQ_EPSILON);
        SpacingConfigurator {
            base_spacing: self.base_spacing,
            overwrite: self.overwrite,
            factors: self.factors.clone(),
            no_op,
        }
    }

    pub fn apply(&self, element: &ElkGraphElementRef) {
        self.build().apply(element);
    }

    pub fn to_visitor(&self) -> Box<dyn IGraphElementVisitor> {
        self.build().to_visitor()
    }
}

fn fuzzy_equals(a: f64, b: f64, epsilon: f64) -> bool {
    (a - b).abs() <= epsilon
}

fn with_properties_mut<R>(
    element: &ElkGraphElementRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    match element {
        ElkGraphElementRef::Node(node) => {
            let mut node_mut = node.borrow_mut();
            let props = node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            f(props)
        }
        ElkGraphElementRef::Edge(edge) => {
            let mut edge_mut = edge.borrow_mut();
            let props = edge_mut.element().properties_mut();
            f(props)
        }
        ElkGraphElementRef::Port(port) => {
            let mut port_mut = port.borrow_mut();
            let props = port_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            f(props)
        }
        ElkGraphElementRef::Label(label) => {
            let mut label_mut = label.borrow_mut();
            let props = label_mut.shape().graph_element().properties_mut();
            f(props)
        }
    }
}
