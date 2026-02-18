use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef;

use crate::org::eclipse::elk::core::options::{CoreOptions, PortLabelPlacement, SizeOptions};
use crate::org::eclipse::elk::core::util::IGraphElementVisitor;

pub struct DeprecatedLayoutOptionReplacer;

impl DeprecatedLayoutOptionReplacer {
    pub fn new() -> Self {
        DeprecatedLayoutOptionReplacer
    }
}

impl Default for DeprecatedLayoutOptionReplacer {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphElementVisitor for DeprecatedLayoutOptionReplacer {
    fn visit(&mut self, element: &ElkGraphElementRef) {
        // Replace PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE with PortLabelPlacement::NextToPortIfPossible.
        let has_next_to_port = with_properties_mut(element, |props| {
            props.has_property(CoreOptions::PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE)
        });
        if has_next_to_port {
            let mut placement = with_properties_mut(element, |props| {
                props
                    .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                    .unwrap_or_else(PortLabelPlacement::outside)
            });
            placement.insert(PortLabelPlacement::NextToPortIfPossible);
            with_properties_mut(element, |props| {
                props.set_property(CoreOptions::PORT_LABELS_PLACEMENT, Some(placement));
                props.set_property(CoreOptions::PORT_LABELS_NEXT_TO_PORT_IF_POSSIBLE, None);
            });
        }

        // Move SizeOptions::SpaceEfficientPortLabels to PortLabelPlacement::SpaceEfficient.
        let mut size_options = with_properties_mut(element, |props| {
            props.get_property(CoreOptions::NODE_SIZE_OPTIONS)
        });
        if let Some(size_options_value) = size_options.as_mut() {
            if size_options_value.contains(&SizeOptions::SpaceEfficientPortLabels) {
                size_options_value.remove(&SizeOptions::SpaceEfficientPortLabels);
                with_properties_mut(element, |props| {
                    props.set_property(
                        CoreOptions::NODE_SIZE_OPTIONS,
                        Some(size_options_value.clone()),
                    );
                });

                let mut placement = with_properties_mut(element, |props| {
                    props
                        .get_property(CoreOptions::PORT_LABELS_PLACEMENT)
                        .unwrap_or_else(PortLabelPlacement::outside)
                });
                placement.insert(PortLabelPlacement::SpaceEfficient);
                with_properties_mut(element, |props| {
                    props.set_property(CoreOptions::PORT_LABELS_PLACEMENT, Some(placement));
                });
            }
        }
    }
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
