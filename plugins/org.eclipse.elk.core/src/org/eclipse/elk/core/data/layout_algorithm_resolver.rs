use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkGraphElementRef, ElkNodeRef};

use crate::org::eclipse::elk::core::data::LayoutMetaDataService;
use crate::org::eclipse::elk::core::options::CoreOptions;
use crate::org::eclipse::elk::core::unsupported_configuration::UnsupportedConfigurationException;
use crate::org::eclipse::elk::core::util::{ElkUtil, IGraphElementVisitor};

pub struct LayoutAlgorithmResolver {
    errors: Vec<UnsupportedConfigurationException>,
}

impl LayoutAlgorithmResolver {
    pub fn new() -> Self {
        LayoutAlgorithmResolver { errors: Vec::new() }
    }

    pub fn errors(&self) -> &[UnsupportedConfigurationException] {
        &self.errors
    }

    pub fn take_errors(&mut self) -> Vec<UnsupportedConfigurationException> {
        std::mem::take(&mut self.errors)
    }

    pub fn default_layout_algorithm_id(&self) -> &'static str {
        "org.eclipse.elk.layered"
    }

    fn resolve_algorithm(
        &self,
        node: &ElkNodeRef,
    ) -> Result<(), UnsupportedConfigurationException> {
        let algorithm_id =
            with_node_properties_mut(node, |props| props.get_property(CoreOptions::ALGORITHM))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());

        if let Some(id) = algorithm_id.as_deref() {
            if self.resolve_and_set_algorithm(id, node) {
                return Ok(());
            }
        }

        if !self.must_resolve(node) {
            return Ok(());
        }

        if algorithm_id.is_none() {
            let default_id = self.default_layout_algorithm_id();
            if self.resolve_and_set_algorithm(default_id, node) {
                return Ok(());
            }

            let mut message = String::from("Unable to load default layout algorithm ");
            message.push_str(default_id);
            message.push_str(" for unconfigured node ");
            ElkUtil::print_element_path(&ElkGraphElementRef::Node(node.clone()), &mut message);
            return Err(UnsupportedConfigurationException::new(message));
        }

        let algorithm_id = algorithm_id.unwrap();
        let mut message = String::from("Layout algorithm '");
        message.push_str(&algorithm_id);
        message.push_str("' not found for ");
        ElkUtil::print_element_path(&ElkGraphElementRef::Node(node.clone()), &mut message);
        Err(UnsupportedConfigurationException::new(message))
    }

    fn resolve_and_set_algorithm(&self, algorithm_id: &str, node: &ElkNodeRef) -> bool {
        let algorithm_data =
            LayoutMetaDataService::get_instance().get_algorithm_data_by_suffix(algorithm_id);

        if let Some(data) = algorithm_data {
            with_node_properties_mut(node, |props| {
                props.set_property(CoreOptions::RESOLVED_ALGORITHM, Some(data));
            });
            true
        } else {
            false
        }
    }

    fn must_resolve(&self, node: &ElkNodeRef) -> bool {
        let (has_resolved, has_children, inside_self_loops) = {
            let mut node_mut = node.borrow_mut();
            let has_children = !node_mut.children().is_empty();
            let props = node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            let has_resolved = props.has_property(CoreOptions::RESOLVED_ALGORITHM);
            let inside_self_loops = props
                .get_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
                .unwrap_or(false);
            (has_resolved, has_children, inside_self_loops)
        };

        !has_resolved && (has_children || inside_self_loops)
    }
}

impl Default for LayoutAlgorithmResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphElementVisitor for LayoutAlgorithmResolver {
    fn visit(&mut self, element: &ElkGraphElementRef) {
        let ElkGraphElementRef::Node(node) = element else {
            return;
        };

        let no_layout = with_node_properties_mut(node, |props| {
            props.get_property(CoreOptions::NO_LAYOUT).unwrap_or(false)
        });
        if no_layout {
            return;
        }

        if let Err(error) = self.resolve_algorithm(node) {
            self.errors.push(error);
        }
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
