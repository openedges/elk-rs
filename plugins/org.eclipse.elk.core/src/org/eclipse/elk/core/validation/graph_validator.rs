use std::collections::HashMap;
use std::rc::Rc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkGraphElementRef, ElkNodeRef,
};

use crate::org::eclipse::elk::core::data::LayoutAlgorithmData;
use crate::org::eclipse::elk::core::options::CoreOptions;
use crate::org::eclipse::elk::core::util::{ElkUtil, IGraphElementVisitor};
use crate::org::eclipse::elk::core::validation::{
    GraphIssue, IValidatingGraphElementVisitor, Severity,
};

pub struct GraphValidator {
    issues: Vec<GraphIssue>,
    algorithm_specific_validators:
        HashMap<LayoutAlgorithmData, Box<dyn IValidatingGraphElementVisitor>>,
    validator_issue_counts: HashMap<LayoutAlgorithmData, usize>,
}

impl GraphValidator {
    pub fn new() -> Self {
        GraphValidator {
            issues: Vec::new(),
            algorithm_specific_validators: HashMap::new(),
            validator_issue_counts: HashMap::new(),
        }
    }

    fn check_edge(&mut self, edge: &ElkEdgeRef) {
        let is_connected = edge.borrow().is_connected();
        if !is_connected {
            self.issues.push(GraphIssue::new(
                Some(ElkGraphElementRef::Edge(edge.clone())),
                "Edge is not connected.",
                Severity::Error,
            ));
        } else {
            let best_container = ElkGraphUtil::find_best_edge_containment(edge);
            let current_container = edge.borrow().containing_node();
            let mismatch = match (&best_container, &current_container) {
                (Some(best), Some(current)) => !Rc::ptr_eq(best, current),
                (Some(_), None) | (None, Some(_)) => true,
                (None, None) => false,
            };
            if mismatch {
                let mut message = String::from("Edge should be contained in ");
                if let Some(best) = best_container {
                    ElkUtil::print_element_path(&ElkGraphElementRef::Node(best), &mut message);
                }
                self.issues.push(GraphIssue::new(
                    Some(ElkGraphElementRef::Edge(edge.clone())),
                    message,
                    Severity::Warning,
                ));
            }
        }

        let sections: Vec<_> = {
            let mut edge_mut = edge.borrow_mut();
            let list = edge_mut.sections();
            (0..list.len())
                .filter_map(|index| list.get(index))
                .collect()
        };

        for section in sections {
            let section_borrow = section.borrow();
            if let Some(incoming) = section_borrow.incoming_shape() {
                if !edge_has_shape(edge, &incoming, true) {
                    self.issues.push(GraphIssue::new(
                        Some(ElkGraphElementRef::Edge(edge.clone())),
                        format!(
                            "{} declared as incoming shape is not a source of this edge.",
                            connectable_shape_kind(&incoming)
                        ),
                        Severity::Error,
                    ));
                }
                if !section_borrow.incoming_sections().is_empty() {
                    self.issues.push(GraphIssue::new(
                        Some(ElkGraphElementRef::Edge(edge.clone())),
                        format!(
                            "An edge section cannot be connected to an {} and other sections at the same time.",
                            connectable_shape_kind(&incoming)
                        ),
                        Severity::Error,
                    ));
                }
            }

            if let Some(outgoing) = section_borrow.outgoing_shape() {
                if !edge_has_shape(edge, &outgoing, false) {
                    self.issues.push(GraphIssue::new(
                        Some(ElkGraphElementRef::Edge(edge.clone())),
                        format!(
                            "{} declared as outgoing shape is not a target of this edge.",
                            connectable_shape_kind(&outgoing)
                        ),
                        Severity::Error,
                    ));
                }
                if !section_borrow.outgoing_sections().is_empty() {
                    self.issues.push(GraphIssue::new(
                        Some(ElkGraphElementRef::Edge(edge.clone())),
                        format!(
                            "An edge section cannot be connected to an {} and other sections at the same time.",
                            connectable_shape_kind(&outgoing)
                        ),
                        Severity::Error,
                    ));
                }
            }
        }
    }

    fn run_algorithm_specific_checks(&mut self, element: &ElkGraphElementRef, parent: &ElkNodeRef) {
        let algo_data = get_resolved_algorithm(parent);
        let Some(algo_data) = algo_data else {
            return;
        };

        let issues_snapshot = {
            let validator = match self.get_validator(&algo_data) {
                Some(validator) => validator,
                None => return,
            };
            validator.visit(element);
            validator.issues().map(|issues| issues.to_vec())
        };

        if let Some(issues) = issues_snapshot {
            let count = self
                .validator_issue_counts
                .entry(algo_data.clone())
                .or_insert(0);
            if *count < issues.len() {
                self.issues.extend(issues[*count..].iter().cloned());
                *count = issues.len();
            }
        }
    }

    fn get_validator(
        &mut self,
        algo_data: &LayoutAlgorithmData,
    ) -> Option<&mut Box<dyn IValidatingGraphElementVisitor>> {
        if self.algorithm_specific_validators.contains_key(algo_data) {
            return self.algorithm_specific_validators.get_mut(algo_data);
        }

        let factory = algo_data.validator_factory().cloned()?;
        let validator = (factory)();
        self.algorithm_specific_validators
            .insert(algo_data.clone(), validator);
        self.validator_issue_counts.insert(algo_data.clone(), 0);
        self.algorithm_specific_validators.get_mut(algo_data)
    }
}

impl Default for GraphValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphElementVisitor for GraphValidator {
    fn visit(&mut self, element: &ElkGraphElementRef) {
        if let ElkGraphElementRef::Edge(edge) = element {
            self.check_edge(edge);
        }

        let parent = ElkGraphUtil::containing_graph(element);
        if let Some(parent_node) = &parent {
            self.run_algorithm_specific_checks(element, parent_node);
        }

        if let ElkGraphElementRef::Node(node) = element {
            let parent_algorithm = parent.as_ref().and_then(get_resolved_algorithm);
            let node_algorithm = get_resolved_algorithm(node);
            let run_for_node = parent.is_none() || parent_algorithm != node_algorithm;
            if run_for_node {
                self.run_algorithm_specific_checks(element, node);
            }
        }
    }

    fn issues(&self) -> Option<&[GraphIssue]> {
        Some(&self.issues)
    }
}

fn get_resolved_algorithm(node: &ElkNodeRef) -> Option<LayoutAlgorithmData> {
    with_node_properties_mut(node, |props| {
        props.get_property(CoreOptions::RESOLVED_ALGORITHM)
    })
}

fn edge_has_shape(edge: &ElkEdgeRef, shape: &ElkConnectableShapeRef, is_source: bool) -> bool {
    let has_shape = {
        let edge_borrow = edge.borrow();
        let list = if is_source {
            edge_borrow.sources_ro()
        } else {
            edge_borrow.targets_ro()
        };
        let result = list.iter().any(|candidate| candidate.ptr_eq(shape));
        result
    };
    has_shape
}

fn connectable_shape_kind(shape: &ElkConnectableShapeRef) -> &'static str {
    match shape {
        ElkConnectableShapeRef::Node(_) => "ElkNode",
        ElkConnectableShapeRef::Port(_) => "ElkPort",
    }
}

fn with_node_properties_mut<R>(
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
