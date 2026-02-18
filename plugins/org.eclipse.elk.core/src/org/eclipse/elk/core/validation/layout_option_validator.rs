use std::any::Any;
use std::cmp::Ordering;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{
    MapPropertyHolder, PropertyValue,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef;

use crate::org::eclipse::elk::core::data::{
    LayoutMetaDataService, LayoutOptionData, LayoutOptionType,
};
use crate::org::eclipse::elk::core::util::exclusive_bounds::{
    ExclusiveLowerBound, ExclusiveUpperBound,
};
use crate::org::eclipse::elk::core::util::IGraphElementVisitor;
use crate::org::eclipse::elk::core::validation::{GraphIssue, Severity};

pub struct LayoutOptionValidator {
    issues: Vec<GraphIssue>,
}

impl LayoutOptionValidator {
    pub fn new() -> Self {
        LayoutOptionValidator { issues: Vec::new() }
    }

    fn check_property(
        &mut self,
        option_data: &LayoutOptionData,
        value: &ArcAny,
        element: &ElkGraphElementRef,
    ) {
        if !is_valid_type(option_data, value.as_ref()) {
            let message = format!(
                "The assigned value {} of the option '{}' does not match the type {}.",
                format_any(value.as_ref()),
                option_label(option_data),
                option_type_name(option_data.option_type()),
            );
            self.issues.push(GraphIssue::new(
                Some(element.clone()),
                message,
                Severity::Error,
            ));
            return;
        }

        if let Some(bound) = option_data.lower_bound() {
            if let Some(ordering) = compare_lower_bound(bound.as_ref(), value.as_ref()) {
                if ordering == Ordering::Greater {
                    let message = format!(
                        "The assigned value {} of the option '{}' is less than the lower bound {}.",
                        format_any(value.as_ref()),
                        option_label(option_data),
                        format_any(bound.as_ref()),
                    );
                    self.issues.push(GraphIssue::new(
                        Some(element.clone()),
                        message,
                        Severity::Error,
                    ));
                    return;
                }
            }
        }

        if let Some(bound) = option_data.upper_bound() {
            if let Some(ordering) = compare_upper_bound(bound.as_ref(), value.as_ref()) {
                if ordering == Ordering::Less {
                    let message = format!(
                        "The assigned value {} of the option '{}' is greater than the upper bound {}.",
                        format_any(value.as_ref()),
                        option_label(option_data),
                        format_any(bound.as_ref()),
                    );
                    self.issues.push(GraphIssue::new(
                        Some(element.clone()),
                        message,
                        Severity::Error,
                    ));
                }
            }
        }
    }
}

impl Default for LayoutOptionValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphElementVisitor for LayoutOptionValidator {
    fn visit(&mut self, element: &ElkGraphElementRef) {
        with_element_properties_mut(element, |properties| {
            let entries: Vec<(String, PropertyValue)> = properties
                .get_all_properties()
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();

            for (property_id, value) in entries {
                let resolved = match value {
                    PropertyValue::Resolved(value) => Some(value),
                    PropertyValue::Proxy(proxy) => {
                        let resolved = proxy.resolve_value(&property_id);
                        if let Some(value) = &resolved {
                            properties.set_property_any(property_id.clone(), Some(value.clone()));
                        }
                        resolved
                    }
                };

                let Some(value) = resolved else {
                    continue;
                };

                let option_data =
                    LayoutMetaDataService::get_instance().get_option_data(&property_id);
                let Some(option_data) = option_data else {
                    continue;
                };

                self.check_property(&option_data, &value, element);
            }
        });
    }

    fn issues(&self) -> Option<&[GraphIssue]> {
        Some(&self.issues)
    }
}

type ArcAny = std::sync::Arc<dyn Any + Send + Sync>;

fn is_valid_type(option_data: &LayoutOptionData, value: &dyn Any) -> bool {
    match option_data.option_type() {
        LayoutOptionType::String => value.is::<String>(),
        LayoutOptionType::Boolean => value.is::<bool>(),
        LayoutOptionType::Int => value.is::<i32>(),
        LayoutOptionType::Double => value.is::<f64>(),
        LayoutOptionType::Enum | LayoutOptionType::EnumSet | LayoutOptionType::Object => {
            option_data
                .value_type_id()
                .map(|type_id| value.type_id() == type_id)
                .unwrap_or(true)
        }
        LayoutOptionType::Undefined => true,
    }
}

fn compare_lower_bound(bound: &dyn Any, value: &dyn Any) -> Option<Ordering> {
    if let Some(exclusive) = bound.downcast_ref::<ExclusiveLowerBound>() {
        let value = value.downcast_ref::<f64>()?;
        return Some(exclusive.compare_to(*value));
    }
    compare_numeric(bound, value)
}

fn compare_upper_bound(bound: &dyn Any, value: &dyn Any) -> Option<Ordering> {
    if let Some(exclusive) = bound.downcast_ref::<ExclusiveUpperBound>() {
        let value = value.downcast_ref::<f64>()?;
        return Some(exclusive.compare_to(*value));
    }
    compare_numeric(bound, value)
}

fn compare_numeric(bound: &dyn Any, value: &dyn Any) -> Option<Ordering> {
    if let Some(bound) = bound.downcast_ref::<i32>() {
        let value = value.downcast_ref::<i32>()?;
        return Some(bound.cmp(value));
    }
    if let Some(bound) = bound.downcast_ref::<f64>() {
        let value = value.downcast_ref::<f64>()?;
        return Some(bound.partial_cmp(value).unwrap_or(Ordering::Equal));
    }
    None
}

fn option_label(option_data: &LayoutOptionData) -> &str {
    if option_data.name().is_empty() {
        option_data.id()
    } else {
        option_data.name()
    }
}

fn option_type_name(option_type: LayoutOptionType) -> &'static str {
    match option_type {
        LayoutOptionType::String => "String",
        LayoutOptionType::Boolean => "Boolean",
        LayoutOptionType::Int => "Integer",
        LayoutOptionType::Double => "Double",
        LayoutOptionType::Enum => "Enum",
        LayoutOptionType::EnumSet => "EnumSet",
        LayoutOptionType::Object => "Object",
        LayoutOptionType::Undefined => "Object",
    }
}

fn format_any(value: &dyn Any) -> String {
    if let Some(value) = value.downcast_ref::<String>() {
        return value.clone();
    }
    if let Some(value) = value.downcast_ref::<bool>() {
        return value.to_string();
    }
    if let Some(value) = value.downcast_ref::<i32>() {
        return value.to_string();
    }
    if let Some(value) = value.downcast_ref::<f64>() {
        return format!("{value:?}");
    }
    if let Some(value) = value.downcast_ref::<ExclusiveLowerBound>() {
        return value.to_string();
    }
    if let Some(value) = value.downcast_ref::<ExclusiveUpperBound>() {
        return value.to_string();
    }
    "<value>".to_string()
}

fn with_element_properties_mut<R>(
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
