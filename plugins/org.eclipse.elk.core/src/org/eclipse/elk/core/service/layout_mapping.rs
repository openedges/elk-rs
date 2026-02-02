use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkGraphElementRef, ElkNodeRef,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;

#[derive(Clone)]
pub struct LayoutMapping {
    properties: MapPropertyHolder,
    element_to_diagram: HashMap<usize, Rc<dyn Any>>,
    diagram_to_element: HashMap<usize, ElkGraphElementRef>,
    layout_graph: Option<ElkNodeRef>,
    parent_element: Option<Rc<dyn Any>>,
    workbench_part: Option<Rc<dyn Any>>,
}

impl LayoutMapping {
    pub fn new(workbench_part: Option<Rc<dyn Any>>) -> Self {
        LayoutMapping {
            properties: MapPropertyHolder::new(),
            element_to_diagram: HashMap::new(),
            diagram_to_element: HashMap::new(),
            layout_graph: None,
            parent_element: None,
            workbench_part,
        }
    }

    pub fn properties(&self) -> &MapPropertyHolder {
        &self.properties
    }

    pub fn properties_mut(&mut self) -> &mut MapPropertyHolder {
        &mut self.properties
    }

    pub fn insert_mapping(&mut self, element: ElkGraphElementRef, diagram: Rc<dyn Any>) {
        let element_key = element_key(&element);
        let diagram_key = Rc::as_ptr(&diagram) as *const () as usize;
        self.element_to_diagram.insert(element_key, diagram.clone());
        self.diagram_to_element.insert(diagram_key, element);
    }

    pub fn diagram_for(&self, element: &ElkGraphElementRef) -> Option<Rc<dyn Any>> {
        self.element_to_diagram
            .get(&element_key(element))
            .cloned()
    }

    pub fn element_for(&self, diagram: &Rc<dyn Any>) -> Option<ElkGraphElementRef> {
        self.diagram_to_element
            .get(&(Rc::as_ptr(diagram) as *const () as usize))
            .cloned()
    }

    pub fn set_layout_graph(&mut self, layout_graph: ElkNodeRef) {
        self.layout_graph = Some(layout_graph);
    }

    pub fn layout_graph(&self) -> Option<ElkNodeRef> {
        self.layout_graph.clone()
    }

    pub fn set_parent_element(&mut self, parent: Option<Rc<dyn Any>>) {
        self.parent_element = parent;
    }

    pub fn parent_element(&self) -> Option<Rc<dyn Any>> {
        self.parent_element.clone()
    }

    pub fn workbench_part(&self) -> Option<Rc<dyn Any>> {
        self.workbench_part.clone()
    }
}

fn element_key(element: &ElkGraphElementRef) -> usize {
    match element {
        ElkGraphElementRef::Node(node) => Rc::as_ptr(node) as usize,
        ElkGraphElementRef::Edge(edge) => Rc::as_ptr(edge) as usize,
        ElkGraphElementRef::Port(port) => Rc::as_ptr(port) as usize,
        ElkGraphElementRef::Label(label) => Rc::as_ptr(label) as usize,
    }
}
