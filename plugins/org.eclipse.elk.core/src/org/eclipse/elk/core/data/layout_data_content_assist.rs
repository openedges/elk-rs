use std::any::Any;

use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    properties::MapPropertyHolder, ElkGraphElementRef, ElkNodeRef, ElkPortRef,
};

use crate::org::eclipse::elk::core::data::{
    ILayoutMetaData, LayoutAlgorithmData, LayoutMetaDataService, LayoutOptionData,
    LayoutOptionTarget, LayoutOptionType, LayoutOptionVisibility,
};
use crate::org::eclipse::elk::core::options::CoreOptions;

pub struct LayoutDataContentAssist;

#[derive(Clone, Debug)]
pub struct Proposal<T> {
    pub proposal: String,
    pub label: Option<String>,
    pub data: Option<T>,
}

impl<T> Proposal<T> {
    pub fn of(proposal: impl Into<String>, label: Option<String>, data: Option<T>) -> Self {
        Proposal {
            proposal: proposal.into(),
            label,
            data,
        }
    }

    pub fn of_data(proposal: impl Into<String>, data: T) -> Self {
        Proposal {
            proposal: proposal.into(),
            label: None,
            data: Some(data),
        }
    }
}

impl LayoutDataContentAssist {
    pub fn get_layout_option_proposals(
        element: &ElkGraphElementRef,
        prefix: &str,
    ) -> Vec<Proposal<LayoutOptionData>> {
        let algorithm = Self::get_algorithm(element);
        match element {
            ElkGraphElementRef::Node(node) => {
                let targets = if node.borrow().is_hierarchical() {
                    vec![LayoutOptionTarget::Nodes, LayoutOptionTarget::Parents]
                } else {
                    vec![LayoutOptionTarget::Nodes]
                };
                Self::get_layout_option_proposals_for_targets(
                    Some(element),
                    algorithm.as_ref(),
                    &targets,
                    prefix,
                )
            }
            ElkGraphElementRef::Edge(_) => Self::get_layout_option_proposals_for_targets(
                Some(element),
                algorithm.as_ref(),
                &[LayoutOptionTarget::Edges],
                prefix,
            ),
            ElkGraphElementRef::Port(_) => Self::get_layout_option_proposals_for_targets(
                Some(element),
                algorithm.as_ref(),
                &[LayoutOptionTarget::Ports],
                prefix,
            ),
            ElkGraphElementRef::Label(_) => Self::get_layout_option_proposals_for_targets(
                Some(element),
                algorithm.as_ref(),
                &[LayoutOptionTarget::Labels],
                prefix,
            ),
        }
    }

    pub fn get_layout_algorithm_proposals(prefix: &str) -> Vec<Proposal<LayoutAlgorithmData>> {
        let service = LayoutMetaDataService::get_instance();
        service
            .get_algorithm_data_list()
            .into_iter()
            .filter_map(|data| {
                matches_prefix(
                    &data,
                    None,
                    |suffix| service.get_algorithm_data_by_suffix(suffix),
                    prefix,
                )
            })
            .collect()
    }

    pub fn get_layout_option_value_proposals(
        option: &LayoutOptionData,
        _prefix: &str,
    ) -> Vec<Proposal<Box<dyn Any>>> {
        let mut proposals = Vec::new();
        match option.option_type() {
            LayoutOptionType::Boolean
            | LayoutOptionType::Enum
            | LayoutOptionType::EnumSet => {
                for choice in option.choices() {
                    proposals.push(Proposal::of(choice.clone(), Some(choice), None));
                }
            }
            LayoutOptionType::Double | LayoutOptionType::Int => {
                let default_value = option.default_default_value();
                if let Some(default_value) = default_value {
                    let mut proposal = String::new();
                    if let Some(value) = default_value.downcast_ref::<i32>() {
                        proposal = value.to_string();
                    } else if let Some(value) = default_value.downcast_ref::<f64>() {
                        proposal = value.to_string();
                    }
                    if !proposal.is_empty() {
                        proposals.push(Proposal::of(
                            proposal,
                            Some(format!("{:?}", option.option_type())),
                            None,
                        ));
                    }
                }
            }
            LayoutOptionType::Object => {
                proposals.push(Proposal::of(
                    String::new(),
                    Some(format!("{:?}", option.option_type())),
                    None,
                ));
            }
            LayoutOptionType::String | LayoutOptionType::Undefined => {}
        }
        proposals
    }

    fn get_layout_option_proposals_for_targets(
        element: Option<&ElkGraphElementRef>,
        algorithm_data: Option<&LayoutAlgorithmData>,
        target_types: &[LayoutOptionTarget],
        prefix: &str,
    ) -> Vec<Proposal<LayoutOptionData>> {
        let service = LayoutMetaDataService::get_instance();
        let algorithm_option_id = CoreOptions::ALGORITHM.id();

        service
            .get_option_data_list()
            .into_iter()
            .filter(|option| {
                option.targets().is_empty()
                    || target_types
                        .iter()
                        .any(|target| option.targets().contains(target))
            })
            .filter(|option| {
                algorithm_data
                    .map(|algorithm| {
                        algorithm.knows_option(option.id()) || option.id() == algorithm_option_id
                    })
                    .unwrap_or(true)
            })
            .filter(|option| {
                element.is_none_or(|element| !has_property_id(element, option.id()))
            })
            .filter(|option| option.visibility() != LayoutOptionVisibility::Hidden)
            .filter_map(|option| {
                matches_prefix(
                    &option,
                    Some(option.group()),
                    |suffix| service.get_option_data_by_suffix(suffix),
                    prefix,
                )
            })
            .collect()
    }

    pub fn get_algorithm(element: &ElkGraphElementRef) -> Option<LayoutAlgorithmData> {
        let relevant_node = get_relevant_node(element)?;
        let algorithm_id = with_node_properties(&relevant_node, |props| {
            props.get_property(CoreOptions::ALGORITHM)
        })
        .filter(|id| !id.trim().is_empty());

        let algorithm_id = algorithm_id.as_deref()?;
        LayoutMetaDataService::get_instance().get_algorithm_data_by_suffix(algorithm_id)
    }
}

fn matches_prefix<T>(
    data: &T,
    group: Option<&str>,
    checker: impl Fn(&str) -> Option<T>,
    prefix: &str,
) -> Option<Proposal<T>>
where
    T: ILayoutMetaData + Clone,
{
    let matches_name = !prefix.is_empty()
        && data
            .name()
            .to_lowercase()
            .contains(&prefix.to_lowercase());
    let id_split: Vec<&str> = data.id().split('.').collect();
    let prefix_split: Option<Vec<&str>> = if matches_name {
        None
    } else {
        Some(prefix.split('.').collect())
    };

    let mut start = id_split.len().saturating_sub(1);
    if let Some(group) = group {
        if !group.is_empty() {
            start = start.saturating_sub(1);
        }
        let group_dots = group.chars().filter(|c| *c == '.').count();
        start = start.saturating_sub(group_dots);
    }

    for i in (0..=start).rev() {
        let suffix_elements = &id_split[i..];
        let suffix = suffix_elements.join(".");
        if checker(&suffix).is_some()
            && (matches_name
                || prefix_split
                    .as_ref()
                    .map(|prefix_split| starts_with(suffix_elements, prefix_split))
                    .unwrap_or(true))
        {
            return Some(Proposal::of_data(suffix, data.clone()));
        }
    }
    None
}

fn starts_with(strings: &[&str], prefix: &[&str]) -> bool {
    if prefix.is_empty() {
        return true;
    }
    if strings.len() < prefix.len() {
        return false;
    }
    for i in 0..=(strings.len() - prefix.len()) {
        let mut matches = true;
        for j in 0..prefix.len() {
            if !strings[i + j].starts_with(prefix[j]) {
                matches = false;
                break;
            }
        }
        if matches {
            return true;
        }
    }
    false
}

fn get_relevant_node(element: &ElkGraphElementRef) -> Option<ElkNodeRef> {
    match element {
        ElkGraphElementRef::Node(node) => {
            if let Some(parent) = node.borrow().parent() {
                Some(parent)
            } else {
                Some(node.clone())
            }
        }
        ElkGraphElementRef::Edge(edge) => edge.borrow().containing_node(),
        ElkGraphElementRef::Port(port) => {
            let parent = port.borrow().parent();
            match parent {
                Some(parent) => {
                    let grandparent = {
                        let parent_ref = parent.borrow();
                        parent_ref.parent()
                    };
                    grandparent.or(Some(parent))
                }
                None => None,
            }
        }
        ElkGraphElementRef::Label(label) => {
            let mut parent = label.borrow().parent();
            while let Some(ElkGraphElementRef::Label(parent_label)) = parent {
                parent = parent_label.borrow().parent();
            }
            parent.and_then(|parent_element| get_relevant_node(&parent_element))
        }
    }
}

fn has_property_id(element: &ElkGraphElementRef, property_id: &str) -> bool {
    match element {
        ElkGraphElementRef::Node(node) => with_node_properties(node, |props| props.has_property_id(property_id)),
        ElkGraphElementRef::Edge(edge) => {
            let mut edge_mut = edge.borrow_mut();
            edge_mut.element().properties_mut().has_property_id(property_id)
        }
        ElkGraphElementRef::Port(port) => with_port_properties(port, |props| props.has_property_id(property_id)),
        ElkGraphElementRef::Label(label) => {
            let mut label_mut = label.borrow_mut();
            label_mut.shape().graph_element().properties_mut().has_property_id(property_id)
        }
    }
}

fn with_node_properties<R>(
    node: &ElkNodeRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    f(props)
}

fn with_port_properties<R>(
    port: &ElkPortRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut port_mut = port.borrow_mut();
    let props = port_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    f(props)
}
