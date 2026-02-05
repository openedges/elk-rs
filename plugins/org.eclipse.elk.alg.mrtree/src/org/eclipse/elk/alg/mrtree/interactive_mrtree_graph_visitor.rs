use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IGraphElementVisitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkGraphElementRef, ElkNodeRef};

use crate::org::eclipse::elk::alg::mrtree::options::{MrTreeOptions, OrderWeighting};

pub struct InteractiveMrTreeGraphVisitor;

impl InteractiveMrTreeGraphVisitor {
    pub fn set_interactive_options(&self, root: &ElkNodeRef) {
        let algorithm = {
            let mut root_mut = root.borrow_mut();
            let props = root_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            props.get_property(CoreOptions::ALGORITHM)
        };
        if let Some(algorithm) = algorithm {
            let has_children = {
                let mut root_mut = root.borrow_mut();
                !root_mut.children().is_empty()
            };
            if MrTreeOptions::ALGORITHM_ID.ends_with(&algorithm) && has_children {
                let mut root_mut = root.borrow_mut();
                let props = root_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut();
                props.set_property(CoreOptions::INTERACTIVE, Some(true));
                props.set_property(MrTreeOptions::WEIGHTING, Some(OrderWeighting::Constraint));
            }
        }
    }
}

impl IGraphElementVisitor for InteractiveMrTreeGraphVisitor {
    fn visit(&mut self, element: &ElkGraphElementRef) {
        if let ElkGraphElementRef::Node(node) = element {
            self.set_interactive_options(node);
        }
    }
}
