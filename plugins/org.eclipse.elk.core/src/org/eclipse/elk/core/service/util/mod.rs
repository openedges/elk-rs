use crate::org::eclipse::elk::core::util::{ElkUtil, IGraphElementVisitor};
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkGraphElementRef, ElkNodeRef};

pub mod progress_monitor_adapter;
pub mod monitored_operation;

pub use progress_monitor_adapter::{IProgressMonitor, ProgressMonitorAdapter};
pub use monitored_operation::{CancelableProgressMonitor, IMonitoredOperation, MonitoredOperation, OperationStatus};

pub struct CompoundGraphElementVisitor {
    graph_visitors: Vec<Box<dyn IGraphElementVisitor>>,
    apply_to_full_graph_first: bool,
}

impl CompoundGraphElementVisitor {
    pub fn new(visitors: Vec<Box<dyn IGraphElementVisitor>>) -> Self {
        CompoundGraphElementVisitor {
            graph_visitors: visitors,
            apply_to_full_graph_first: false,
        }
    }

    pub fn new_with_mode(
        apply_to_full_graph_first: bool,
        visitors: Vec<Box<dyn IGraphElementVisitor>>,
    ) -> Self {
        CompoundGraphElementVisitor {
            graph_visitors: visitors,
            apply_to_full_graph_first,
        }
    }

    pub fn add_graph_visitors(&mut self, visitors: Vec<Box<dyn IGraphElementVisitor>>) {
        self.graph_visitors.extend(visitors);
    }

    pub fn graph_visitors(&self) -> &Vec<Box<dyn IGraphElementVisitor>> {
        &self.graph_visitors
    }
}

impl IGraphElementVisitor for CompoundGraphElementVisitor {
    fn visit(&mut self, element: &ElkGraphElementRef) {
        if self.apply_to_full_graph_first {
            if let ElkGraphElementRef::Node(node) = element {
                if is_root_node(node) {
                    let mut visitor_refs: Vec<&mut dyn IGraphElementVisitor> = self
                        .graph_visitors
                        .iter_mut()
                        .map(|v| v.as_mut() as &mut dyn IGraphElementVisitor)
                        .collect();
                    ElkUtil::apply_visitors(node, &mut visitor_refs);
                }
            }
            return;
        }

        for visitor in self.graph_visitors.iter_mut() {
            visitor.visit(element);
        }
    }
}

fn is_root_node(node: &ElkNodeRef) -> bool {
    node.borrow().parent().is_none()
}
