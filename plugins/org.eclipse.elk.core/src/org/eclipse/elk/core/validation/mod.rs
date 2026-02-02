use std::fmt;

use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef;

use crate::org::eclipse::elk::core::util::IGraphElementVisitor;

pub mod graph_validator;
pub mod layout_option_validator;

pub use graph_validator::GraphValidator;
pub use layout_option_validator::LayoutOptionValidator;

pub trait IValidatingGraphElementVisitor: IGraphElementVisitor {}

impl<T: IGraphElementVisitor> IValidatingGraphElementVisitor for T {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn user_string(self) -> &'static str {
        match self {
            Severity::Error => "Error",
            Severity::Warning => "Warning",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "ERROR"),
            Severity::Warning => write!(f, "WARNING"),
        }
    }
}

#[derive(Clone)]
pub struct GraphIssue {
    element: Option<ElkGraphElementRef>,
    message: String,
    severity: Severity,
}

impl GraphIssue {
    pub fn new(
        element: Option<ElkGraphElementRef>,
        message: impl Into<String>,
        severity: Severity,
    ) -> Self {
        GraphIssue {
            element,
            message: message.into(),
            severity,
        }
    }

    pub fn element(&self) -> Option<&ElkGraphElementRef> {
        self.element.as_ref()
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }
}

pub struct GraphValidationException {
    message: String,
    issues: Vec<GraphIssue>,
}

impl fmt::Debug for GraphValidationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GraphValidationException")
            .field("message", &self.message)
            .field("issue_count", &self.issues.len())
            .finish()
    }
}

impl GraphValidationException {
    pub fn new(message: impl Into<String>, issues: Vec<GraphIssue>) -> Self {
        GraphValidationException {
            message: message.into(),
            issues,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn issues(&self) -> &[GraphIssue] {
        &self.issues
    }
}

impl fmt::Display for GraphValidationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for GraphValidationException {}
