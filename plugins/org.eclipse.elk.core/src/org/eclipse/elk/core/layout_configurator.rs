use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{
    MapPropertyHolder, Property, PropertyValue,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef;

use crate::org::eclipse::elk::core::data::{LayoutMetaDataService, LayoutOptionTarget};
use crate::org::eclipse::elk::core::util::IGraphElementVisitor;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum LayoutConfiguratorClass {
    GraphElement,
    Shape,
    Label,
    ConnectableShape,
    Node,
    Port,
    Edge,
}

pub trait IOptionFilter: Send + Sync {
    fn accept(&self, element: &ElkGraphElementRef, property_id: &str) -> bool;
}

impl<F> IOptionFilter for F
where
    F: Fn(&ElkGraphElementRef, &str) -> bool + Send + Sync,
{
    fn accept(&self, element: &ElkGraphElementRef, property_id: &str) -> bool {
        (self)(element, property_id)
    }
}

pub trait IPropertyHolderOptionFilter: Send + Sync {
    fn accept(&self, holder: &MapPropertyHolder, property_id: &str) -> bool;
}

impl<F> IPropertyHolderOptionFilter for F
where
    F: Fn(&MapPropertyHolder, &str) -> bool + Send + Sync,
{
    fn accept(&self, holder: &MapPropertyHolder, property_id: &str) -> bool {
        (self)(holder, property_id)
    }
}

pub static ADD_LAYOUT_CONFIG: LazyLock<Property<Arc<LayoutConfigurator>>> =
    LazyLock::new(|| Property::new("org.eclipse.elk.addLayoutConfig"));

pub static NO_OVERWRITE: LazyLock<Arc<dyn IOptionFilter>> = LazyLock::new(|| {
    Arc::new(|element: &ElkGraphElementRef, property_id: &str| {
        !element_has_property(element, property_id)
    })
});

pub static NO_OVERWRITE_HOLDER: LazyLock<Arc<dyn IPropertyHolderOptionFilter>> =
    LazyLock::new(|| {
        Arc::new(|holder: &MapPropertyHolder, property_id: &str| {
            !holder.has_property_id(property_id)
        })
    });

pub static OPTION_TARGET_FILTER: LazyLock<Arc<dyn IOptionFilter>> = LazyLock::new(|| {
    Arc::new(|element: &ElkGraphElementRef, property_id: &str| {
        let option_data = LayoutMetaDataService::get_instance().get_option_data(property_id);
        let Some(option_data) = option_data else {
            return true;
        };

        let targets = option_data.targets();
        match element {
            ElkGraphElementRef::Node(node) => {
                let is_hierarchical = node.borrow().is_hierarchical();
                if is_hierarchical {
                    targets.contains(&LayoutOptionTarget::Nodes)
                        || targets.contains(&LayoutOptionTarget::Parents)
                } else {
                    targets.contains(&LayoutOptionTarget::Nodes)
                }
            }
            ElkGraphElementRef::Edge(_) => targets.contains(&LayoutOptionTarget::Edges),
            ElkGraphElementRef::Port(_) => targets.contains(&LayoutOptionTarget::Ports),
            ElkGraphElementRef::Label(_) => targets.contains(&LayoutOptionTarget::Labels),
        }
    })
});

#[derive(Clone, Default)]
pub struct LayoutConfigurator {
    element_option_map: HashMap<usize, MapPropertyHolder>,
    class_option_map: HashMap<LayoutConfiguratorClass, MapPropertyHolder>,
    clear_layout: bool,
    option_filters: Vec<Arc<dyn IOptionFilter>>,
}

impl LayoutConfigurator {
    pub fn new() -> Self {
        LayoutConfigurator::default()
    }

    pub fn is_clear_layout(&self) -> bool {
        self.clear_layout
    }

    pub fn set_clear_layout(&mut self, do_clear_layout: bool) -> &mut Self {
        self.clear_layout = do_clear_layout;
        self
    }

    pub fn add_filter(&mut self, filter: Arc<dyn IOptionFilter>) -> &mut Self {
        self.option_filters.push(filter);
        self
    }

    pub fn filters(&self) -> &[Arc<dyn IOptionFilter>] {
        &self.option_filters
    }

    pub fn configure_element(&mut self, element: &ElkGraphElementRef) -> &mut MapPropertyHolder {
        let key = element_key(element);
        self.element_option_map.entry(key).or_default()
    }

    pub fn configure_node(
        &mut self,
        node: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef,
    ) -> &mut MapPropertyHolder {
        self.configure_element(&ElkGraphElementRef::Node(node.clone()))
    }

    pub fn configure_port(
        &mut self,
        port: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkPortRef,
    ) -> &mut MapPropertyHolder {
        self.configure_element(&ElkGraphElementRef::Port(port.clone()))
    }

    pub fn configure_edge(
        &mut self,
        edge: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeRef,
    ) -> &mut MapPropertyHolder {
        self.configure_element(&ElkGraphElementRef::Edge(edge.clone()))
    }

    pub fn configure_label(
        &mut self,
        label: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkLabelRef,
    ) -> &mut MapPropertyHolder {
        self.configure_element(&ElkGraphElementRef::Label(label.clone()))
    }

    pub fn get_properties_element(
        &self,
        element: &ElkGraphElementRef,
    ) -> Option<&MapPropertyHolder> {
        let key = element_key(element);
        self.element_option_map.get(&key)
    }

    pub fn configure_class(
        &mut self,
        element_class: LayoutConfiguratorClass,
    ) -> &mut MapPropertyHolder {
        self.class_option_map.entry(element_class).or_default()
    }

    pub fn get_properties_class(
        &self,
        element_class: LayoutConfiguratorClass,
    ) -> Option<&MapPropertyHolder> {
        self.class_option_map.get(&element_class)
    }

    fn apply_properties(&self, element: &ElkGraphElementRef, properties: &MapPropertyHolder) {
        let entries: Vec<_> = properties
            .get_all_properties()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();

        for (property_id, value) in entries {
            let accept = self
                .option_filters
                .iter()
                .all(|filter| filter.accept(element, &property_id));
            if !accept {
                continue;
            }

            let cloned = clone_property_value(&value);
            with_element_properties_mut(element, |holder| match cloned {
                PropertyValue::Resolved(value) => {
                    holder.set_property_any(property_id.clone(), Some(value));
                }
                PropertyValue::Proxy(proxy) => {
                    holder.set_property_proxy(property_id.clone(), proxy);
                }
            });
        }
    }

    fn find_class_options(&self, element: &ElkGraphElementRef) -> MapPropertyHolder {
        let mut combined = MapPropertyHolder::new();

        if let Some(holder) = self
            .class_option_map
            .get(&LayoutConfiguratorClass::GraphElement)
        {
            combined.copy_properties(holder);
        }

        match element {
            ElkGraphElementRef::Label(_) => {
                if let Some(holder) = self.class_option_map.get(&LayoutConfiguratorClass::Shape) {
                    combined.copy_properties(holder);
                }
                if let Some(holder) = self.class_option_map.get(&LayoutConfiguratorClass::Label) {
                    combined.copy_properties(holder);
                }
                return combined;
            }
            ElkGraphElementRef::Node(_) => {
                if let Some(holder) = self.class_option_map.get(&LayoutConfiguratorClass::Shape) {
                    combined.copy_properties(holder);
                }
                if let Some(holder) = self
                    .class_option_map
                    .get(&LayoutConfiguratorClass::ConnectableShape)
                {
                    combined.copy_properties(holder);
                }
                if let Some(holder) = self.class_option_map.get(&LayoutConfiguratorClass::Node) {
                    combined.copy_properties(holder);
                }
                return combined;
            }
            ElkGraphElementRef::Port(_) => {
                if let Some(holder) = self.class_option_map.get(&LayoutConfiguratorClass::Shape) {
                    combined.copy_properties(holder);
                }
                if let Some(holder) = self
                    .class_option_map
                    .get(&LayoutConfiguratorClass::ConnectableShape)
                {
                    combined.copy_properties(holder);
                }
                if let Some(holder) = self.class_option_map.get(&LayoutConfiguratorClass::Port) {
                    combined.copy_properties(holder);
                }
                return combined;
            }
            ElkGraphElementRef::Edge(_) => {
                if let Some(holder) = self.class_option_map.get(&LayoutConfiguratorClass::Edge) {
                    combined.copy_properties(holder);
                }
            }
        }

        combined
    }

    pub fn override_with(&mut self, other: &LayoutConfigurator) -> &mut Self {
        for (key, holder) in &other.element_option_map {
            let this_holder = self.element_option_map.entry(*key).or_default();
            this_holder.copy_properties(holder);
        }

        for (key, holder) in &other.class_option_map {
            let this_holder = self.class_option_map.entry(*key).or_default();
            this_holder.copy_properties(holder);
        }

        self.clear_layout = other.clear_layout;
        self.option_filters.clear();
        self.option_filters
            .extend(other.option_filters.iter().cloned());
        self
    }
}

impl IGraphElementVisitor for LayoutConfigurator {
    fn visit(&mut self, element: &ElkGraphElementRef) {
        if self.clear_layout {
            with_element_properties_mut(element, |holder| holder.clear());
        }

        let mut combined = self.find_class_options(element);
        if let Some(holder) = self.get_properties_element(element) {
            combined.copy_properties(holder);
        }
        self.apply_properties(element, &combined);
    }
}

fn element_key(element: &ElkGraphElementRef) -> usize {
    match element {
        ElkGraphElementRef::Node(node) => std::rc::Rc::as_ptr(node) as usize,
        ElkGraphElementRef::Edge(edge) => std::rc::Rc::as_ptr(edge) as usize,
        ElkGraphElementRef::Port(port) => std::rc::Rc::as_ptr(port) as usize,
        ElkGraphElementRef::Label(label) => std::rc::Rc::as_ptr(label) as usize,
    }
}

fn element_has_property(element: &ElkGraphElementRef, property_id: &str) -> bool {
    with_element_properties_mut(element, |holder| holder.has_property_id(property_id))
}

fn clone_property_value(value: &PropertyValue) -> PropertyValue {
    match value {
        PropertyValue::Resolved(value) => {
            if let Some(cloned) = ElkReflect::clone_any(value.as_ref()) {
                PropertyValue::Resolved(Arc::from(cloned))
            } else {
                PropertyValue::Resolved(value.clone())
            }
        }
        PropertyValue::Proxy(proxy) => PropertyValue::Proxy(proxy.clone()),
    }
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
