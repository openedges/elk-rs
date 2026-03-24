use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub mod elk_graph_arena;
pub mod properties;
pub mod util;

pub use elk_graph_arena::{
    EBendId, EConnectableId, EEdgeId, ELabelId, ENodeId, EPortId, ESectionId,
    ElkGraphArena,
};

use properties::MapPropertyHolder;

pub type ElkNodeRef = Rc<RefCell<ElkNode>>;
pub type ElkNodeWeak = Weak<RefCell<ElkNode>>;

pub type ElkPortRef = Rc<RefCell<ElkPort>>;
pub type ElkPortWeak = Weak<RefCell<ElkPort>>;

pub type ElkEdgeRef = Rc<RefCell<ElkEdge>>;
pub type ElkEdgeWeak = Weak<RefCell<ElkEdge>>;

pub type ElkLabelRef = Rc<RefCell<ElkLabel>>;
pub type ElkLabelWeak = Weak<RefCell<ElkLabel>>;

pub type ElkEdgeSectionRef = Rc<RefCell<ElkEdgeSection>>;
pub type ElkEdgeSectionWeak = Weak<RefCell<ElkEdgeSection>>;

pub type ElkBendPointRef = Rc<RefCell<ElkBendPoint>>;

#[derive(Clone)]
pub enum ElkGraphElementRef {
    Node(ElkNodeRef),
    Edge(ElkEdgeRef),
    Port(ElkPortRef),
    Label(ElkLabelRef),
}

#[derive(Clone)]
pub enum ElkGraphElementWeak {
    Node(ElkNodeWeak),
    Edge(ElkEdgeWeak),
    Port(ElkPortWeak),
    Label(ElkLabelWeak),
}

impl ElkGraphElementWeak {
    pub fn upgrade(&self) -> Option<ElkGraphElementRef> {
        match self {
            ElkGraphElementWeak::Node(node) => node.upgrade().map(ElkGraphElementRef::Node),
            ElkGraphElementWeak::Edge(edge) => edge.upgrade().map(ElkGraphElementRef::Edge),
            ElkGraphElementWeak::Port(port) => port.upgrade().map(ElkGraphElementRef::Port),
            ElkGraphElementWeak::Label(label) => label.upgrade().map(ElkGraphElementRef::Label),
        }
    }
}

impl ElkGraphElementRef {
    pub fn downgrade(&self) -> ElkGraphElementWeak {
        match self {
            ElkGraphElementRef::Node(node) => ElkGraphElementWeak::Node(Rc::downgrade(node)),
            ElkGraphElementRef::Edge(edge) => ElkGraphElementWeak::Edge(Rc::downgrade(edge)),
            ElkGraphElementRef::Port(port) => ElkGraphElementWeak::Port(Rc::downgrade(port)),
            ElkGraphElementRef::Label(label) => ElkGraphElementWeak::Label(Rc::downgrade(label)),
        }
    }
}

#[derive(Clone)]
pub enum ElkConnectableShapeRef {
    Node(ElkNodeRef),
    Port(ElkPortRef),
}

#[derive(Clone)]
pub enum ElkConnectableShapeWeak {
    Node(ElkNodeWeak),
    Port(ElkPortWeak),
}

impl ElkConnectableShapeWeak {
    pub fn upgrade(&self) -> Option<ElkConnectableShapeRef> {
        match self {
            ElkConnectableShapeWeak::Node(node) => node.upgrade().map(ElkConnectableShapeRef::Node),
            ElkConnectableShapeWeak::Port(port) => port.upgrade().map(ElkConnectableShapeRef::Port),
        }
    }
}

impl ElkConnectableShapeRef {
    pub fn ptr_eq(&self, other: &ElkConnectableShapeRef) -> bool {
        match (self, other) {
            (ElkConnectableShapeRef::Node(a), ElkConnectableShapeRef::Node(b)) => Rc::ptr_eq(a, b),
            (ElkConnectableShapeRef::Port(a), ElkConnectableShapeRef::Port(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }

    pub fn downgrade(&self) -> ElkConnectableShapeWeak {
        match self {
            ElkConnectableShapeRef::Node(node) => {
                ElkConnectableShapeWeak::Node(Rc::downgrade(node))
            }
            ElkConnectableShapeRef::Port(port) => {
                ElkConnectableShapeWeak::Port(Rc::downgrade(port))
            }
        }
    }
}

impl From<ElkConnectableShapeRef> for ElkGraphElementRef {
    fn from(shape: ElkConnectableShapeRef) -> Self {
        match shape {
            ElkConnectableShapeRef::Node(node) => ElkGraphElementRef::Node(node),
            ElkConnectableShapeRef::Port(port) => ElkGraphElementRef::Port(port),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EdgeEndpointKind {
    Source,
    Target,
}

pub struct LabelList {
    owner: ElkGraphElementWeak,
    items: Vec<ElkLabelRef>,
}

impl LabelList {
    fn new(owner: ElkGraphElementWeak) -> Self {
        LabelList {
            owner,
            items: Vec::new(),
        }
    }

    pub fn add(&mut self, label: ElkLabelRef) {
        let owner = match self.owner.upgrade() {
            Some(owner) => owner,
            None => return,
        };
        let current_parent = label.borrow().parent();
        if let Some(current_parent) = current_parent {
            if same_graph_element(&current_parent, &owner) {
                self.add_internal(label);
                return;
            }
            remove_label_from_parent(&current_parent, &label);
        }
        {
            let mut label_mut = label.borrow_mut();
            label_mut.parent = Some(owner.downgrade());
        }
        self.add_internal(label);
    }

    fn add_internal(&mut self, label: ElkLabelRef) {
        if !self.items.iter().any(|item| Rc::ptr_eq(item, &label)) {
            self.items.push(label);
        }
    }

    fn remove_internal(&mut self, label: &ElkLabelRef) {
        if let Some(index) = self.items.iter().position(|item| Rc::ptr_eq(item, label)) {
            self.items.remove(index);
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<ElkLabelRef> {
        self.items.get(index).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ElkLabelRef> {
        self.items.iter()
    }
}

pub struct NodeChildList {
    owner: ElkNodeWeak,
    items: Vec<ElkNodeRef>,
}

impl NodeChildList {
    fn new(owner: ElkNodeWeak) -> Self {
        NodeChildList {
            owner,
            items: Vec::new(),
        }
    }

    pub fn add(&mut self, child: ElkNodeRef) {
        let owner = match self.owner.upgrade() {
            Some(owner) => owner,
            None => return,
        };
        let current_parent = child.borrow().parent();
        if let Some(current_parent) = current_parent {
            if Rc::ptr_eq(&current_parent, &owner) {
                self.add_internal(child);
                return;
            }
            current_parent.borrow_mut().children.remove_internal(&child);
        }
        {
            let mut child_mut = child.borrow_mut();
            child_mut.parent = Some(Rc::downgrade(&owner));
        }
        self.add_internal(child);
    }

    fn add_internal(&mut self, child: ElkNodeRef) {
        if !self.items.iter().any(|item| Rc::ptr_eq(item, &child)) {
            self.items.push(child);
        }
    }

    fn remove_internal(&mut self, child: &ElkNodeRef) {
        if let Some(index) = self.items.iter().position(|item| Rc::ptr_eq(item, child)) {
            self.items.remove(index);
        }
    }

    pub fn remove_at(&mut self, index: usize) -> Option<ElkNodeRef> {
        if index >= self.items.len() {
            return None;
        }
        let child = self.items.remove(index);
        child.borrow_mut().parent = None;
        Some(child)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<ElkNodeRef> {
        self.items.get(index).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ElkNodeRef> {
        self.items.iter()
    }
}

pub struct NodePortList {
    owner: ElkNodeWeak,
    items: Vec<ElkPortRef>,
}

impl NodePortList {
    fn new(owner: ElkNodeWeak) -> Self {
        NodePortList {
            owner,
            items: Vec::new(),
        }
    }

    pub fn add(&mut self, port: ElkPortRef) {
        let owner = match self.owner.upgrade() {
            Some(owner) => owner,
            None => return,
        };
        let current_parent = port.borrow().parent();
        if let Some(current_parent) = current_parent {
            if Rc::ptr_eq(&current_parent, &owner) {
                self.add_internal(port);
                return;
            }
            current_parent.borrow_mut().ports.remove_internal(&port);
        }
        {
            let mut port_mut = port.borrow_mut();
            port_mut.parent = Some(Rc::downgrade(&owner));
        }
        self.add_internal(port);
    }

    fn add_internal(&mut self, port: ElkPortRef) {
        if !self.items.iter().any(|item| Rc::ptr_eq(item, &port)) {
            self.items.push(port);
        }
    }

    fn remove_internal(&mut self, port: &ElkPortRef) {
        if let Some(index) = self.items.iter().position(|item| Rc::ptr_eq(item, port)) {
            self.items.remove(index);
        }
    }

    pub fn remove_at(&mut self, index: usize) -> Option<ElkPortRef> {
        if index >= self.items.len() {
            return None;
        }
        let port = self.items.remove(index);
        port.borrow_mut().parent = None;
        Some(port)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<ElkPortRef> {
        self.items.get(index).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ElkPortRef> {
        self.items.iter()
    }

    pub fn sort_by<F>(&mut self, mut comparator: F)
    where
        F: FnMut(&ElkPortRef, &ElkPortRef) -> std::cmp::Ordering,
    {
        self.items.sort_by(|a, b| comparator(a, b));
    }
}

pub struct NodeEdgeList {
    owner: ElkNodeWeak,
    items: Vec<ElkEdgeRef>,
}

impl NodeEdgeList {
    fn new(owner: ElkNodeWeak) -> Self {
        NodeEdgeList {
            owner,
            items: Vec::new(),
        }
    }

    pub fn add(&mut self, edge: ElkEdgeRef) {
        let owner = match self.owner.upgrade() {
            Some(owner) => owner,
            None => return,
        };
        let current_node = edge.borrow().containing_node();
        if let Some(current_node) = current_node {
            if Rc::ptr_eq(&current_node, &owner) {
                self.add_internal(edge);
                return;
            }
            current_node
                .borrow_mut()
                .contained_edges
                .remove_internal(&edge);
        }
        {
            let mut edge_mut = edge.borrow_mut();
            edge_mut.containing_node = Some(Rc::downgrade(&owner));
        }
        self.add_internal(edge);
    }

    fn add_internal(&mut self, edge: ElkEdgeRef) {
        if !self.items.iter().any(|item| Rc::ptr_eq(item, &edge)) {
            self.items.push(edge);
        }
    }

    fn remove_internal(&mut self, edge: &ElkEdgeRef) {
        if let Some(index) = self.items.iter().position(|item| Rc::ptr_eq(item, edge)) {
            self.items.remove(index);
        }
    }

    pub fn remove_at(&mut self, index: usize) -> Option<ElkEdgeRef> {
        if index >= self.items.len() {
            return None;
        }
        let edge = self.items.remove(index);
        edge.borrow_mut().containing_node = None;
        Some(edge)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<ElkEdgeRef> {
        self.items.get(index).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ElkEdgeRef> {
        self.items.iter()
    }
}

pub struct EdgeEndpointList {
    owner: ElkEdgeWeak,
    kind: EdgeEndpointKind,
    items: Vec<ElkConnectableShapeRef>,
}

impl EdgeEndpointList {
    fn new(owner: ElkEdgeWeak, kind: EdgeEndpointKind) -> Self {
        EdgeEndpointList {
            owner,
            kind,
            items: Vec::new(),
        }
    }

    pub fn add(&mut self, shape: ElkConnectableShapeRef) {
        if self.items.iter().any(|item| item.ptr_eq(&shape)) {
            return;
        }
        self.items.push(shape.clone());
        if let Some(edge) = self.owner.upgrade() {
            match self.kind {
                EdgeEndpointKind::Source => match shape {
                    ElkConnectableShapeRef::Node(node) => {
                        node.borrow_mut().connectable.add_outgoing_internal(&edge)
                    }
                    ElkConnectableShapeRef::Port(port) => {
                        port.borrow_mut().connectable.add_outgoing_internal(&edge)
                    }
                },
                EdgeEndpointKind::Target => match shape {
                    ElkConnectableShapeRef::Node(node) => {
                        node.borrow_mut().connectable.add_incoming_internal(&edge)
                    }
                    ElkConnectableShapeRef::Port(port) => {
                        port.borrow_mut().connectable.add_incoming_internal(&edge)
                    }
                },
            }
        }
    }

    fn add_internal(&mut self, shape: ElkConnectableShapeRef) {
        if !self.items.iter().any(|item| item.ptr_eq(&shape)) {
            self.items.push(shape);
        }
    }

    fn remove_internal(&mut self, shape: &ElkConnectableShapeRef) {
        if let Some(index) = self.items.iter().position(|item| item.ptr_eq(shape)) {
            self.items.remove(index);
        }
    }

    pub fn remove_at(&mut self, index: usize) -> Option<ElkConnectableShapeRef> {
        if index >= self.items.len() {
            return None;
        }
        let shape = self.items.remove(index);
        if let Some(edge) = self.owner.upgrade() {
            match self.kind {
                EdgeEndpointKind::Source => match shape {
                    ElkConnectableShapeRef::Node(ref node) => node
                        .borrow_mut()
                        .connectable
                        .remove_outgoing_internal(&edge),
                    ElkConnectableShapeRef::Port(ref port) => port
                        .borrow_mut()
                        .connectable
                        .remove_outgoing_internal(&edge),
                },
                EdgeEndpointKind::Target => match shape {
                    ElkConnectableShapeRef::Node(ref node) => node
                        .borrow_mut()
                        .connectable
                        .remove_incoming_internal(&edge),
                    ElkConnectableShapeRef::Port(ref port) => port
                        .borrow_mut()
                        .connectable
                        .remove_incoming_internal(&edge),
                },
            }
        }
        Some(shape)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<ElkConnectableShapeRef> {
        self.items.get(index).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ElkConnectableShapeRef> {
        self.items.iter()
    }
}

pub struct ConnectableEdgeList {
    owner: ElkConnectableShapeWeak,
    kind: EdgeEndpointKind,
    items: Vec<ElkEdgeWeak>,
}

impl ConnectableEdgeList {
    fn new(owner: ElkConnectableShapeWeak, kind: EdgeEndpointKind) -> Self {
        ConnectableEdgeList {
            owner,
            kind,
            items: Vec::new(),
        }
    }

    pub fn add(&mut self, edge: ElkEdgeRef) {
        if self
            .items
            .iter()
            .any(|item| item.upgrade().is_some_and(|e| Rc::ptr_eq(&e, &edge)))
        {
            return;
        }
        let owner = match self.owner.upgrade() {
            Some(owner) => owner,
            None => return,
        };
        self.items.push(Rc::downgrade(&edge));
        let mut edge_mut = edge.borrow_mut();
        match self.kind {
            EdgeEndpointKind::Source => edge_mut.sources.add_internal(owner),
            EdgeEndpointKind::Target => edge_mut.targets.add_internal(owner),
        }
    }

    fn add_internal(&mut self, edge: &ElkEdgeRef) {
        if !self
            .items
            .iter()
            .any(|item| item.upgrade().is_some_and(|e| Rc::ptr_eq(&e, edge)))
        {
            self.items.push(Rc::downgrade(edge));
        }
    }

    fn remove_internal(&mut self, edge: &ElkEdgeRef) {
        if let Some(index) = self.items.iter().position(|item| {
            item.upgrade()
                .is_some_and(|existing| Rc::ptr_eq(&existing, edge))
        }) {
            self.items.remove(index);
        }
    }

    pub fn len(&self) -> usize {
        self.items
            .iter()
            .filter(|edge| edge.upgrade().is_some())
            .count()
    }

    pub fn is_empty(&self) -> bool {
        !self.items.iter().any(|edge| edge.upgrade().is_some())
    }

    pub fn iter(&self) -> impl Iterator<Item = ElkEdgeRef> + '_ {
        self.items.iter().filter_map(|edge| edge.upgrade())
    }
}

pub struct EdgeSectionList {
    owner: ElkEdgeWeak,
    items: Vec<ElkEdgeSectionRef>,
}

impl EdgeSectionList {
    fn new(owner: ElkEdgeWeak) -> Self {
        EdgeSectionList {
            owner,
            items: Vec::new(),
        }
    }

    pub fn add(&mut self, section: ElkEdgeSectionRef) {
        let owner = match self.owner.upgrade() {
            Some(owner) => owner,
            None => return,
        };
        let current_parent = section.borrow().parent();
        if let Some(current_parent) = current_parent {
            if Rc::ptr_eq(&current_parent, &owner) {
                self.add_internal(section);
                return;
            }
            current_parent
                .borrow_mut()
                .sections
                .remove_internal(&section);
        }
        {
            let mut section_mut = section.borrow_mut();
            section_mut.parent = Some(Rc::downgrade(&owner));
        }
        self.add_internal(section);
    }

    fn add_internal(&mut self, section: ElkEdgeSectionRef) {
        if !self.items.iter().any(|item| Rc::ptr_eq(item, &section)) {
            self.items.push(section);
        }
    }

    fn remove_internal(&mut self, section: &ElkEdgeSectionRef) {
        if let Some(index) = self.items.iter().position(|item| Rc::ptr_eq(item, section)) {
            self.items.remove(index);
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<ElkEdgeSectionRef> {
        self.items.get(index).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ElkEdgeSectionRef> {
        self.items.iter()
    }

    pub fn clear(&mut self) {
        for section in self.items.drain(..) {
            section.borrow_mut().parent = None;
        }
    }

    pub fn retain_last(&mut self) {
        if self.items.len() <= 1 {
            return;
        }
        let last = self.items.pop().expect("list not empty");
        for section in self.items.drain(..) {
            section.borrow_mut().parent = None;
        }
        self.items.push(last);
    }
}

pub struct ElkGraphElement {
    labels: LabelList,
    identifier: Option<String>,
    properties: MapPropertyHolder,
}

impl ElkGraphElement {
    fn new(owner: ElkGraphElementWeak) -> Self {
        ElkGraphElement {
            labels: LabelList::new(owner),
            identifier: None,
            properties: MapPropertyHolder::new(),
        }
    }

    pub fn labels(&mut self) -> &mut LabelList {
        &mut self.labels
    }

    pub fn identifier(&self) -> Option<&str> {
        self.identifier.as_deref()
    }

    pub fn set_identifier(&mut self, identifier: Option<String>) {
        self.identifier = identifier;
    }

    pub fn properties(&self) -> &MapPropertyHolder {
        &self.properties
    }

    pub fn properties_mut(&mut self) -> &mut MapPropertyHolder {
        &mut self.properties
    }
}

pub struct ElkShape {
    element: ElkGraphElement,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl ElkShape {
    fn new(owner: ElkGraphElementWeak) -> Self {
        ElkShape {
            element: ElkGraphElement::new(owner),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn graph_element(&mut self) -> &mut ElkGraphElement {
        &mut self.element
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn set_x(&mut self, value: f64) {
        self.x = value;
    }

    pub fn set_y(&mut self, value: f64) {
        self.y = value;
    }

    pub fn set_width(&mut self, value: f64) {
        self.width = value;
    }

    pub fn set_height(&mut self, value: f64) {
        self.height = value;
    }

    pub fn set_dimensions(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
    }

    pub fn set_location(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }
}

pub struct ElkConnectableShape {
    shape: ElkShape,
    outgoing_edges: ConnectableEdgeList,
    incoming_edges: ConnectableEdgeList,
}

impl ElkConnectableShape {
    fn new(owner: ElkConnectableShapeWeak, element_owner: ElkGraphElementWeak) -> Self {
        ElkConnectableShape {
            shape: ElkShape::new(element_owner),
            outgoing_edges: ConnectableEdgeList::new(owner.clone(), EdgeEndpointKind::Source),
            incoming_edges: ConnectableEdgeList::new(owner, EdgeEndpointKind::Target),
        }
    }

    pub fn shape(&mut self) -> &mut ElkShape {
        &mut self.shape
    }

    pub fn outgoing_edges(&self) -> &ConnectableEdgeList {
        &self.outgoing_edges
    }

    pub fn incoming_edges(&self) -> &ConnectableEdgeList {
        &self.incoming_edges
    }

    fn add_outgoing_internal(&mut self, edge: &ElkEdgeRef) {
        self.outgoing_edges.add_internal(edge);
    }

    fn remove_outgoing_internal(&mut self, edge: &ElkEdgeRef) {
        self.outgoing_edges.remove_internal(edge);
    }

    fn add_incoming_internal(&mut self, edge: &ElkEdgeRef) {
        self.incoming_edges.add_internal(edge);
    }

    fn remove_incoming_internal(&mut self, edge: &ElkEdgeRef) {
        self.incoming_edges.remove_internal(edge);
    }
}

pub struct ElkNode {
    connectable: ElkConnectableShape,
    ports: NodePortList,
    children: NodeChildList,
    parent: Option<ElkNodeWeak>,
    contained_edges: NodeEdgeList,
}

impl ElkNode {
    pub fn new() -> ElkNodeRef {
        Rc::new_cyclic(|weak| {
            RefCell::new(ElkNode {
                connectable: ElkConnectableShape::new(
                    ElkConnectableShapeWeak::Node(weak.clone()),
                    ElkGraphElementWeak::Node(weak.clone()),
                ),
                ports: NodePortList::new(weak.clone()),
                children: NodeChildList::new(weak.clone()),
                parent: None,
                contained_edges: NodeEdgeList::new(weak.clone()),
            })
        })
    }

    pub fn parent(&self) -> Option<ElkNodeRef> {
        self.parent.as_ref().and_then(|parent| parent.upgrade())
    }

    pub fn set_parent(node: &ElkNodeRef, parent: Option<ElkNodeRef>) {
        let current_parent = node.borrow().parent();
        if let (Some(current_parent), Some(new_parent)) = (&current_parent, &parent) {
            if Rc::ptr_eq(current_parent, new_parent) {
                return;
            }
        }

        if let Some(current_parent) = current_parent {
            current_parent.borrow_mut().children.remove_internal(node);
        }

        {
            let mut node_mut = node.borrow_mut();
            node_mut.parent = parent.as_ref().map(Rc::downgrade);
        }

        if let Some(parent) = parent {
            parent.borrow_mut().children.add_internal(node.clone());
        }
    }

    pub fn ports(&mut self) -> &mut NodePortList {
        &mut self.ports
    }

    pub fn children(&mut self) -> &mut NodeChildList {
        &mut self.children
    }

    pub fn contained_edges(&mut self) -> &mut NodeEdgeList {
        &mut self.contained_edges
    }

    pub fn is_hierarchical(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn connectable(&mut self) -> &mut ElkConnectableShape {
        &mut self.connectable
    }
}

pub struct ElkPort {
    connectable: ElkConnectableShape,
    parent: Option<ElkNodeWeak>,
}

impl ElkPort {
    pub fn new() -> ElkPortRef {
        Rc::new_cyclic(|weak| {
            RefCell::new(ElkPort {
                connectable: ElkConnectableShape::new(
                    ElkConnectableShapeWeak::Port(weak.clone()),
                    ElkGraphElementWeak::Port(weak.clone()),
                ),
                parent: None,
            })
        })
    }

    pub fn parent(&self) -> Option<ElkNodeRef> {
        self.parent.as_ref().and_then(|parent| parent.upgrade())
    }

    pub fn set_parent(port: &ElkPortRef, parent: Option<ElkNodeRef>) {
        let current_parent = port.borrow().parent();
        if let (Some(current_parent), Some(new_parent)) = (&current_parent, &parent) {
            if Rc::ptr_eq(current_parent, new_parent) {
                return;
            }
        }

        if let Some(current_parent) = current_parent {
            current_parent.borrow_mut().ports.remove_internal(port);
        }

        {
            let mut port_mut = port.borrow_mut();
            port_mut.parent = parent.as_ref().map(Rc::downgrade);
        }

        if let Some(parent) = parent {
            parent.borrow_mut().ports.add_internal(port.clone());
        }
    }

    pub fn connectable(&mut self) -> &mut ElkConnectableShape {
        &mut self.connectable
    }
}

pub struct ElkEdge {
    element: ElkGraphElement,
    containing_node: Option<ElkNodeWeak>,
    sources: EdgeEndpointList,
    targets: EdgeEndpointList,
    sections: EdgeSectionList,
}

impl ElkEdge {
    pub fn new() -> ElkEdgeRef {
        Rc::new_cyclic(|weak| {
            RefCell::new(ElkEdge {
                element: ElkGraphElement::new(ElkGraphElementWeak::Edge(weak.clone())),
                containing_node: None,
                sources: EdgeEndpointList::new(weak.clone(), EdgeEndpointKind::Source),
                targets: EdgeEndpointList::new(weak.clone(), EdgeEndpointKind::Target),
                sections: EdgeSectionList::new(weak.clone()),
            })
        })
    }

    pub fn containing_node(&self) -> Option<ElkNodeRef> {
        self.containing_node
            .as_ref()
            .and_then(|node| node.upgrade())
    }

    pub fn set_containing_node(edge: &ElkEdgeRef, node: Option<ElkNodeRef>) {
        let current_node = edge.borrow().containing_node();
        if let (Some(current_node), Some(new_node)) = (&current_node, &node) {
            if Rc::ptr_eq(current_node, new_node) {
                return;
            }
        }

        if let Some(current_node) = current_node {
            current_node
                .borrow_mut()
                .contained_edges
                .remove_internal(edge);
        }

        {
            let mut edge_mut = edge.borrow_mut();
            edge_mut.containing_node = node.as_ref().map(Rc::downgrade);
        }

        if let Some(node) = node {
            node.borrow_mut().contained_edges.add_internal(edge.clone());
        }
    }

    pub fn sources(&mut self) -> &mut EdgeEndpointList {
        &mut self.sources
    }

    pub fn targets(&mut self) -> &mut EdgeEndpointList {
        &mut self.targets
    }

    pub fn sources_ro(&self) -> &EdgeEndpointList {
        &self.sources
    }

    pub fn targets_ro(&self) -> &EdgeEndpointList {
        &self.targets
    }

    pub fn sections(&mut self) -> &mut EdgeSectionList {
        &mut self.sections
    }

    pub fn element(&mut self) -> &mut ElkGraphElement {
        &mut self.element
    }

    pub fn add_source(edge: &ElkEdgeRef, shape: ElkConnectableShapeRef) {
        let mut edge_mut = edge.borrow_mut();
        if edge_mut
            .sources
            .items
            .iter()
            .any(|item| item.ptr_eq(&shape))
        {
            return;
        }
        edge_mut.sources.add_internal(shape.clone());
        drop(edge_mut);
        match shape {
            ElkConnectableShapeRef::Node(node) => {
                node.borrow_mut().connectable.add_outgoing_internal(edge)
            }
            ElkConnectableShapeRef::Port(port) => {
                port.borrow_mut().connectable.add_outgoing_internal(edge)
            }
        }
    }

    pub fn add_target(edge: &ElkEdgeRef, shape: ElkConnectableShapeRef) {
        let mut edge_mut = edge.borrow_mut();
        if edge_mut
            .targets
            .items
            .iter()
            .any(|item| item.ptr_eq(&shape))
        {
            return;
        }
        edge_mut.targets.add_internal(shape.clone());
        drop(edge_mut);
        match shape {
            ElkConnectableShapeRef::Node(node) => {
                node.borrow_mut().connectable.add_incoming_internal(edge)
            }
            ElkConnectableShapeRef::Port(port) => {
                port.borrow_mut().connectable.add_incoming_internal(edge)
            }
        }
    }

    pub fn remove_source(edge: &ElkEdgeRef, shape: &ElkConnectableShapeRef) {
        {
            let mut edge_mut = edge.borrow_mut();
            edge_mut.sources.remove_internal(shape);
        }
        match shape {
            ElkConnectableShapeRef::Node(node) => {
                node.borrow_mut().connectable.remove_outgoing_internal(edge)
            }
            ElkConnectableShapeRef::Port(port) => {
                port.borrow_mut().connectable.remove_outgoing_internal(edge)
            }
        }
    }

    pub fn remove_target(edge: &ElkEdgeRef, shape: &ElkConnectableShapeRef) {
        {
            let mut edge_mut = edge.borrow_mut();
            edge_mut.targets.remove_internal(shape);
        }
        match shape {
            ElkConnectableShapeRef::Node(node) => {
                node.borrow_mut().connectable.remove_incoming_internal(edge)
            }
            ElkConnectableShapeRef::Port(port) => {
                port.borrow_mut().connectable.remove_incoming_internal(edge)
            }
        }
    }

    pub fn is_hyperedge(&self) -> bool {
        self.sources.len() > 1 || self.targets.len() > 1
    }

    pub fn is_connected(&self) -> bool {
        !self.sources.is_empty() && !self.targets.is_empty()
    }

    pub fn is_selfloop(&self) -> bool {
        if !self.is_connected() {
            return false;
        }
        let mut iter = self.sources.iter().chain(self.targets.iter());
        let first = match iter.next() {
            Some(shape) => shape,
            None => return false,
        };
        let first_node =
            match crate::org::eclipse::elk::graph::util::ElkGraphUtil::connectable_shape_to_node(
                first,
            ) {
                Some(node) => node,
                None => return false,
            };
        iter.all(|shape| {
            crate::org::eclipse::elk::graph::util::ElkGraphUtil::connectable_shape_to_node(shape)
                .map(|node| Rc::ptr_eq(&node, &first_node))
                .unwrap_or(false)
        })
    }

    pub fn is_hierarchical(&self) -> bool {
        if self.sources.is_empty() && self.targets.is_empty() {
            return false;
        }
        let mut parent: Option<ElkNodeRef> = None;
        for shape in self.sources.iter().chain(self.targets.iter()) {
            let node =
                match crate::org::eclipse::elk::graph::util::ElkGraphUtil::connectable_shape_to_node(
                    shape,
                ) {
                    Some(node) => node,
                    None => return true,
                };
            let node_parent = node.borrow().parent();
            match (&parent, &node_parent) {
                (None, _) => parent = node_parent,
                (Some(existing), Some(candidate)) => {
                    if !Rc::ptr_eq(existing, candidate) {
                        return true;
                    }
                }
                (Some(_), None) => return true,
            }
        }
        false
    }
}

pub struct ElkLabel {
    shape: ElkShape,
    parent: Option<ElkGraphElementWeak>,
    text: String,
}

impl ElkLabel {
    pub fn new() -> ElkLabelRef {
        Rc::new_cyclic(|weak| {
            RefCell::new(ElkLabel {
                shape: ElkShape::new(ElkGraphElementWeak::Label(weak.clone())),
                parent: None,
                text: String::new(),
            })
        })
    }

    pub fn parent(&self) -> Option<ElkGraphElementRef> {
        self.parent.as_ref().and_then(|parent| parent.upgrade())
    }

    pub fn set_parent(label: &ElkLabelRef, parent: Option<ElkGraphElementRef>) {
        let current_parent = label.borrow().parent();
        if let (Some(current_parent), Some(new_parent)) = (&current_parent, &parent) {
            if std::mem::discriminant(current_parent) == std::mem::discriminant(new_parent) {
                // fall through to allow changes between different instances of same kind
            }
        }

        if let Some(current_parent) = current_parent {
            remove_label_from_parent(&current_parent, label);
        }

        {
            let mut label_mut = label.borrow_mut();
            label_mut.parent = parent.as_ref().map(|parent| parent.downgrade());
        }

        if let Some(parent) = parent {
            add_label_to_parent(&parent, label.clone());
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn shape(&mut self) -> &mut ElkShape {
        &mut self.shape
    }
}

pub struct ElkEdgeSection {
    properties: MapPropertyHolder,
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    bend_points: Vec<ElkBendPointRef>,
    parent: Option<ElkEdgeWeak>,
    outgoing_shape: Option<ElkConnectableShapeWeak>,
    incoming_shape: Option<ElkConnectableShapeWeak>,
    outgoing_sections: Vec<ElkEdgeSectionWeak>,
    incoming_sections: Vec<ElkEdgeSectionWeak>,
    identifier: Option<String>,
}

impl ElkEdgeSection {
    pub fn new() -> ElkEdgeSectionRef {
        Rc::new(RefCell::new(ElkEdgeSection {
            properties: MapPropertyHolder::new(),
            start_x: 0.0,
            start_y: 0.0,
            end_x: 0.0,
            end_y: 0.0,
            bend_points: Vec::new(),
            parent: None,
            outgoing_shape: None,
            incoming_shape: None,
            outgoing_sections: Vec::new(),
            incoming_sections: Vec::new(),
            identifier: None,
        }))
    }

    pub fn set_parent(section: &ElkEdgeSectionRef, parent: Option<ElkEdgeRef>) {
        let current_parent = section.borrow().parent();
        if let (Some(current_parent), Some(new_parent)) = (&current_parent, &parent) {
            if Rc::ptr_eq(current_parent, new_parent) {
                return;
            }
        }

        if let Some(current_parent) = current_parent {
            current_parent
                .borrow_mut()
                .sections
                .remove_internal(section);
        }

        {
            let mut section_mut = section.borrow_mut();
            section_mut.parent = parent.as_ref().map(Rc::downgrade);
        }

        if let Some(parent) = parent {
            parent.borrow_mut().sections.add_internal(section.clone());
        }
    }

    pub fn parent(&self) -> Option<ElkEdgeRef> {
        self.parent.as_ref().and_then(|parent| parent.upgrade())
    }

    pub fn outgoing_shape(&self) -> Option<ElkConnectableShapeRef> {
        self.outgoing_shape
            .as_ref()
            .and_then(|shape| shape.upgrade())
    }

    pub fn set_outgoing_shape(&mut self, shape: Option<ElkConnectableShapeRef>) {
        self.outgoing_shape = shape.as_ref().map(|value| value.downgrade());
    }

    pub fn incoming_shape(&self) -> Option<ElkConnectableShapeRef> {
        self.incoming_shape
            .as_ref()
            .and_then(|shape| shape.upgrade())
    }

    pub fn set_incoming_shape(&mut self, shape: Option<ElkConnectableShapeRef>) {
        self.incoming_shape = shape.as_ref().map(|value| value.downgrade());
    }

    pub fn outgoing_sections(&self) -> Vec<ElkEdgeSectionRef> {
        self.outgoing_sections
            .iter()
            .filter_map(|section| section.upgrade())
            .collect()
    }

    pub fn set_outgoing_sections(&mut self, sections: Vec<ElkEdgeSectionRef>) {
        self.outgoing_sections = sections
            .into_iter()
            .map(|section| Rc::downgrade(&section))
            .collect();
    }

    pub fn incoming_sections(&self) -> Vec<ElkEdgeSectionRef> {
        self.incoming_sections
            .iter()
            .filter_map(|section| section.upgrade())
            .collect()
    }

    pub fn set_incoming_sections(&mut self, sections: Vec<ElkEdgeSectionRef>) {
        self.incoming_sections = sections
            .into_iter()
            .map(|section| Rc::downgrade(&section))
            .collect();
    }

    pub fn start_x(&self) -> f64 {
        self.start_x
    }

    pub fn start_y(&self) -> f64 {
        self.start_y
    }

    pub fn end_x(&self) -> f64 {
        self.end_x
    }

    pub fn end_y(&self) -> f64 {
        self.end_y
    }

    pub fn set_start_x(&mut self, value: f64) {
        self.start_x = value;
    }

    pub fn set_start_y(&mut self, value: f64) {
        self.start_y = value;
    }

    pub fn set_end_x(&mut self, value: f64) {
        self.end_x = value;
    }

    pub fn set_end_y(&mut self, value: f64) {
        self.end_y = value;
    }

    pub fn bend_points(&mut self) -> &mut Vec<ElkBendPointRef> {
        &mut self.bend_points
    }

    pub fn set_identifier(&mut self, identifier: Option<String>) {
        self.identifier = identifier;
    }

    pub fn identifier(&self) -> Option<&str> {
        self.identifier.as_deref()
    }

    pub fn properties(&self) -> &MapPropertyHolder {
        &self.properties
    }

    pub fn properties_mut(&mut self) -> &mut MapPropertyHolder {
        &mut self.properties
    }
}

pub struct ElkBendPoint {
    x: f64,
    y: f64,
}

impl ElkBendPoint {
    pub fn new() -> ElkBendPointRef {
        Rc::new(RefCell::new(ElkBendPoint { x: 0.0, y: 0.0 }))
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn set_x(&mut self, value: f64) {
        self.x = value;
    }

    pub fn set_y(&mut self, value: f64) {
        self.y = value;
    }
}

fn same_graph_element(a: &ElkGraphElementRef, b: &ElkGraphElementRef) -> bool {
    match (a, b) {
        (ElkGraphElementRef::Node(a), ElkGraphElementRef::Node(b)) => Rc::ptr_eq(a, b),
        (ElkGraphElementRef::Edge(a), ElkGraphElementRef::Edge(b)) => Rc::ptr_eq(a, b),
        (ElkGraphElementRef::Port(a), ElkGraphElementRef::Port(b)) => Rc::ptr_eq(a, b),
        (ElkGraphElementRef::Label(a), ElkGraphElementRef::Label(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

fn add_label_to_parent(parent: &ElkGraphElementRef, label: ElkLabelRef) {
    match parent {
        ElkGraphElementRef::Node(node) => node
            .borrow_mut()
            .connectable
            .shape
            .graph_element()
            .labels()
            .add_internal(label),
        ElkGraphElementRef::Edge(edge) => edge.borrow_mut().element.labels().add_internal(label),
        ElkGraphElementRef::Port(port) => port
            .borrow_mut()
            .connectable
            .shape
            .graph_element()
            .labels()
            .add_internal(label),
        ElkGraphElementRef::Label(label_parent) => label_parent
            .borrow_mut()
            .shape
            .graph_element()
            .labels()
            .add_internal(label),
    }
}

fn remove_label_from_parent(parent: &ElkGraphElementRef, label: &ElkLabelRef) {
    match parent {
        ElkGraphElementRef::Node(node) => node
            .borrow_mut()
            .connectable
            .shape
            .graph_element()
            .labels()
            .remove_internal(label),
        ElkGraphElementRef::Edge(edge) => edge.borrow_mut().element.labels().remove_internal(label),
        ElkGraphElementRef::Port(port) => port
            .borrow_mut()
            .connectable
            .shape
            .graph_element()
            .labels()
            .remove_internal(label),
        ElkGraphElementRef::Label(label_parent) => label_parent
            .borrow_mut()
            .shape
            .graph_element()
            .labels()
            .remove_internal(label),
    }
}

pub struct ElkGraphFactory;

impl ElkGraphFactory {
    pub fn instance() -> &'static ElkGraphFactory {
        static INSTANCE: ElkGraphFactory = ElkGraphFactory;
        &INSTANCE
    }

    pub fn create_elk_label(&self) -> ElkLabelRef {
        ElkLabel::new()
    }

    pub fn create_elk_node(&self) -> ElkNodeRef {
        ElkNode::new()
    }

    pub fn create_elk_port(&self) -> ElkPortRef {
        ElkPort::new()
    }

    pub fn create_elk_edge(&self) -> ElkEdgeRef {
        ElkEdge::new()
    }

    pub fn create_elk_bend_point(&self) -> ElkBendPointRef {
        ElkBendPoint::new()
    }

    pub fn create_elk_edge_section(&self) -> ElkEdgeSectionRef {
        ElkEdgeSection::new()
    }

    pub fn get_elk_graph_package(&self) -> &'static ElkGraphPackage {
        ElkGraphPackage::instance()
    }
}

pub struct ElkGraphPackage;

impl ElkGraphPackage {
    pub const E_NAME: &'static str = "graph";
    pub const E_NS_URI: &'static str = "http://www.eclipse.org/elk/ElkGraph";
    pub const E_NS_PREFIX: &'static str = "graph";

    pub fn instance() -> &'static ElkGraphPackage {
        static INSTANCE: ElkGraphPackage = ElkGraphPackage;
        &INSTANCE
    }
}
