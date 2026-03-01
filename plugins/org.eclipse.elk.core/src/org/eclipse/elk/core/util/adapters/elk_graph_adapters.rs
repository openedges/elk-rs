use std::any::Any;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::rc::{Rc, Weak};
use std::sync::LazyLock;

static TRACE_CORE_PORT_SORT: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_CORE_PORT_SORT").is_some());

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkLabelRef, ElkNodeRef, ElkPortRef, ElkShape,
};

use crate::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector};
use crate::org::eclipse::elk::core::options::{CoreOptions, LabelSide, PortConstraints, PortSide};

use super::{
    EdgeAdapter, GraphAdapter, GraphElementAdapter, LabelAdapter, NodeAdapter, PortAdapter,
};

static OFFSET_PROXY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::from_property(CoreOptions::PORT_BORDER_OFFSET, 0.0));

pub struct ElkGraphAdapters;

impl ElkGraphAdapters {
    pub fn adapt(graph: ElkNodeRef) -> ElkGraphAdapter {
        ElkGraphAdapter::new(graph)
    }

    pub fn adapt_single_node(node: ElkNodeRef) -> ElkNodeAdapter {
        let parent = node.borrow().parent();
        match parent {
            Some(parent) => {
                let graph_adapter = ElkGraphAdapter::new(parent);
                ElkNodeAdapter::new_with_parent_strong(graph_adapter, node)
            }
            None => ElkNodeAdapter::new(node),
        }
    }

    pub fn adapt_single_port(port: ElkPortRef) -> ElkPortAdapter {
        ElkPortAdapter::new(port)
    }
}

#[derive(Clone)]
pub struct ElkGraphAdapter {
    inner: Rc<ElkGraphAdapterInner>,
}

struct ElkGraphAdapterInner {
    element: ElkNodeRef,
    child_nodes: RefCell<Option<Vec<ElkNodeAdapter>>>,
    volatile_id: Cell<i32>,
}

impl ElkGraphAdapter {
    fn new(node: ElkNodeRef) -> Self {
        ElkGraphAdapter {
            inner: Rc::new(ElkGraphAdapterInner {
                element: node,
                child_nodes: RefCell::new(None),
                volatile_id: Cell::new(0),
            }),
        }
    }

    fn from_inner(inner: Rc<ElkGraphAdapterInner>) -> Self {
        ElkGraphAdapter { inner }
    }

    fn downgrade(&self) -> Weak<ElkGraphAdapterInner> {
        Rc::downgrade(&self.inner)
    }

    pub fn element(&self) -> ElkNodeRef {
        self.inner.element.clone()
    }
}

#[derive(Clone)]
pub struct ElkNodeAdapter {
    inner: Rc<ElkNodeAdapterInner>,
}

enum ParentGraphRef {
    Strong(ElkGraphAdapter),
    Weak(Weak<ElkGraphAdapterInner>),
}

impl ParentGraphRef {
    fn upgrade(&self) -> Option<ElkGraphAdapter> {
        match self {
            ParentGraphRef::Strong(adapter) => Some(adapter.clone()),
            ParentGraphRef::Weak(adapter) => adapter.upgrade().map(ElkGraphAdapter::from_inner),
        }
    }
}

struct ElkNodeAdapterInner {
    element: ElkNodeRef,
    parent_graph: Option<ParentGraphRef>,
    label_adapters: RefCell<Option<Vec<ElkLabelAdapter>>>,
    port_adapters: RefCell<Option<Vec<ElkPortAdapter>>>,
    incoming_edge_adapters: RefCell<Option<Vec<ElkEdgeAdapter>>>,
    outgoing_edge_adapters: RefCell<Option<Vec<ElkEdgeAdapter>>>,
    volatile_id: Cell<i32>,
}

impl ElkNodeAdapter {
    fn new(node: ElkNodeRef) -> Self {
        ElkNodeAdapter {
            inner: Rc::new(ElkNodeAdapterInner {
                element: node,
                parent_graph: None,
                label_adapters: RefCell::new(None),
                port_adapters: RefCell::new(None),
                incoming_edge_adapters: RefCell::new(None),
                outgoing_edge_adapters: RefCell::new(None),
                volatile_id: Cell::new(0),
            }),
        }
    }

    fn new_with_parent_weak(parent: &ElkGraphAdapter, node: ElkNodeRef) -> Self {
        ElkNodeAdapter {
            inner: Rc::new(ElkNodeAdapterInner {
                element: node,
                parent_graph: Some(ParentGraphRef::Weak(parent.downgrade())),
                label_adapters: RefCell::new(None),
                port_adapters: RefCell::new(None),
                incoming_edge_adapters: RefCell::new(None),
                outgoing_edge_adapters: RefCell::new(None),
                volatile_id: Cell::new(0),
            }),
        }
    }

    fn new_with_parent_strong(parent: ElkGraphAdapter, node: ElkNodeRef) -> Self {
        ElkNodeAdapter {
            inner: Rc::new(ElkNodeAdapterInner {
                element: node,
                parent_graph: Some(ParentGraphRef::Strong(parent)),
                label_adapters: RefCell::new(None),
                port_adapters: RefCell::new(None),
                incoming_edge_adapters: RefCell::new(None),
                outgoing_edge_adapters: RefCell::new(None),
                volatile_id: Cell::new(0),
            }),
        }
    }

    pub fn element(&self) -> ElkNodeRef {
        self.inner.element.clone()
    }
}

#[derive(Clone)]
pub struct ElkLabelAdapter {
    inner: Rc<ElkLabelAdapterInner>,
}

struct ElkLabelAdapterInner {
    element: ElkLabelRef,
    volatile_id: Cell<i32>,
}

impl ElkLabelAdapter {
    fn new(label: ElkLabelRef) -> Self {
        ElkLabelAdapter {
            inner: Rc::new(ElkLabelAdapterInner {
                element: label,
                volatile_id: Cell::new(0),
            }),
        }
    }
}

#[derive(Clone)]
pub struct ElkPortAdapter {
    inner: Rc<ElkPortAdapterInner>,
}

struct ElkPortAdapterInner {
    element: ElkPortRef,
    label_adapters: RefCell<Option<Vec<ElkLabelAdapter>>>,
    incoming_edge_adapters: RefCell<Option<Vec<ElkEdgeAdapter>>>,
    outgoing_edge_adapters: RefCell<Option<Vec<ElkEdgeAdapter>>>,
    volatile_id: Cell<i32>,
}

impl ElkPortAdapter {
    fn new(port: ElkPortRef) -> Self {
        ElkPortAdapter {
            inner: Rc::new(ElkPortAdapterInner {
                element: port,
                label_adapters: RefCell::new(None),
                incoming_edge_adapters: RefCell::new(None),
                outgoing_edge_adapters: RefCell::new(None),
                volatile_id: Cell::new(0),
            }),
        }
    }

    pub fn element(&self) -> ElkPortRef {
        self.inner.element.clone()
    }
}

#[derive(Clone)]
pub struct ElkEdgeAdapter {
    inner: Rc<ElkEdgeAdapterInner>,
}

struct ElkEdgeAdapterInner {
    element: ElkEdgeRef,
    label_adapters: RefCell<Option<Vec<ElkLabelAdapter>>>,
}

impl ElkEdgeAdapter {
    fn new(edge: ElkEdgeRef) -> Self {
        ElkEdgeAdapter {
            inner: Rc::new(ElkEdgeAdapterInner {
                element: edge,
                label_adapters: RefCell::new(None),
            }),
        }
    }

    pub fn element(&self) -> ElkEdgeRef {
        self.inner.element.clone()
    }
}

fn with_node_shape_mut<R>(node: &ElkNodeRef, f: impl FnOnce(&mut ElkShape) -> R) -> R {
    let mut node_mut = node.borrow_mut();
    f(node_mut.connectable().shape())
}

fn with_port_shape_mut<R>(port: &ElkPortRef, f: impl FnOnce(&mut ElkShape) -> R) -> R {
    let mut port_mut = port.borrow_mut();
    f(port_mut.connectable().shape())
}

fn with_label_shape_mut<R>(label: &ElkLabelRef, f: impl FnOnce(&mut ElkShape) -> R) -> R {
    let mut label_mut = label.borrow_mut();
    f(label_mut.shape())
}

fn with_node_properties_mut<R>(
    node: &ElkNodeRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut node_mut = node.borrow_mut();
    f(node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut())
}

fn with_port_properties_mut<R>(
    port: &ElkPortRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut port_mut = port.borrow_mut();
    f(port_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut())
}

fn with_label_properties_mut<R>(
    label: &ElkLabelRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut label_mut = label.borrow_mut();
    f(label_mut.shape().graph_element().properties_mut())
}

fn with_edge_properties_mut<R>(
    edge: &ElkEdgeRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut edge_mut = edge.borrow_mut();
    f(edge_mut.element().properties_mut())
}

fn get_property_with_offset<P: Clone + Send + Sync + 'static>(
    props: &mut MapPropertyHolder,
    prop: &Property<P>,
) -> Option<P> {
    if prop.id() == CoreOptions::PORT_BORDER_OFFSET.id() {
        if let Some(value) = props.get_property(&OFFSET_PROXY) {
            let boxed: Box<dyn Any> = Box::new(value);
            if let Ok(casted) = boxed.downcast::<P>() {
                return Some(*casted);
            }
            return None;
        }
    }
    props.get_property(prop)
}

impl GraphElementAdapter<ElkNodeRef> for ElkGraphAdapter {
    fn get_size(&self) -> KVector {
        with_node_shape_mut(&self.inner.element, |shape| {
            KVector::with_values(shape.width(), shape.height())
        })
    }

    fn set_size(&self, size: KVector) {
        with_node_shape_mut(&self.inner.element, |shape| {
            shape.set_width(size.x);
            shape.set_height(size.y);
        });
    }

    fn get_position(&self) -> KVector {
        with_node_shape_mut(&self.inner.element, |shape| {
            KVector::with_values(shape.x(), shape.y())
        })
    }

    fn set_position(&self, pos: KVector) {
        with_node_shape_mut(&self.inner.element, |shape| {
            shape.set_x(pos.x);
            shape.set_y(pos.y);
        });
    }

    fn get_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> Option<P> {
        with_node_properties_mut(&self.inner.element, |props| {
            get_property_with_offset(props, prop)
        })
    }

    fn has_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> bool {
        with_node_properties_mut(&self.inner.element, |props| props.has_property(prop))
    }

    fn get_volatile_id(&self) -> i32 {
        self.inner.volatile_id.get()
    }

    fn set_volatile_id(&self, volatile_id: i32) {
        self.inner.volatile_id.set(volatile_id);
    }
}

impl GraphAdapter<ElkNodeRef> for ElkGraphAdapter {
    type Node = ElkNodeRef;
    type NodeAdapter = ElkNodeAdapter;

    fn get_nodes(&self) -> Vec<Self::NodeAdapter> {
        let mut cache = self.inner.child_nodes.borrow_mut();
        if cache.is_none() {
            let children: Vec<ElkNodeRef> = {
                let mut node_mut = self.inner.element.borrow_mut();
                node_mut.children().iter().cloned().collect()
            };
            let adapters = children
                .into_iter()
                .map(|child| ElkNodeAdapter::new_with_parent_weak(self, child))
                .collect::<Vec<_>>();
            *cache = Some(adapters);
        }
        cache.as_ref().cloned().unwrap_or_default()
    }
}

impl GraphElementAdapter<ElkNodeRef> for ElkNodeAdapter {
    fn get_size(&self) -> KVector {
        with_node_shape_mut(&self.inner.element, |shape| {
            KVector::with_values(shape.width(), shape.height())
        })
    }

    fn set_size(&self, size: KVector) {
        with_node_shape_mut(&self.inner.element, |shape| {
            shape.set_width(size.x);
            shape.set_height(size.y);
        });
    }

    fn get_position(&self) -> KVector {
        with_node_shape_mut(&self.inner.element, |shape| {
            KVector::with_values(shape.x(), shape.y())
        })
    }

    fn set_position(&self, pos: KVector) {
        with_node_shape_mut(&self.inner.element, |shape| {
            shape.set_x(pos.x);
            shape.set_y(pos.y);
        });
    }

    fn get_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> Option<P> {
        with_node_properties_mut(&self.inner.element, |props| {
            get_property_with_offset(props, prop)
        })
    }

    fn has_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> bool {
        with_node_properties_mut(&self.inner.element, |props| props.has_property(prop))
    }

    fn get_volatile_id(&self) -> i32 {
        self.inner.volatile_id.get()
    }

    fn set_volatile_id(&self, volatile_id: i32) {
        self.inner.volatile_id.set(volatile_id);
    }
}

impl NodeAdapter<ElkNodeRef> for ElkNodeAdapter {
    type Graph = ElkGraphAdapter;
    type Label = ElkLabelRef;
    type LabelAdapter = ElkLabelAdapter;
    type Port = ElkPortRef;
    type PortAdapter = ElkPortAdapter;
    type Edge = ElkEdgeRef;
    type EdgeAdapter = ElkEdgeAdapter;

    fn get_graph(&self) -> Option<Self::Graph> {
        self.inner
            .parent_graph
            .as_ref()
            .and_then(|parent| parent.upgrade())
    }

    fn get_labels(&self) -> Vec<Self::LabelAdapter> {
        let mut cache = self.inner.label_adapters.borrow_mut();
        if cache.is_none() {
            let labels: Vec<ElkLabelRef> = {
                let mut node_mut = self.inner.element.borrow_mut();
                node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect()
            };
            let adapters = labels.into_iter().map(ElkLabelAdapter::new).collect();
            *cache = Some(adapters);
        }
        cache.as_ref().cloned().unwrap_or_default()
    }

    fn get_ports(&self) -> Vec<Self::PortAdapter> {
        let mut cache = self.inner.port_adapters.borrow_mut();
        if cache.is_none() {
            let ports: Vec<ElkPortRef> = {
                let mut node_mut = self.inner.element.borrow_mut();
                node_mut.ports().iter().cloned().collect()
            };
            let adapters = ports.into_iter().map(ElkPortAdapter::new).collect();
            *cache = Some(adapters);
        }
        cache.as_ref().cloned().unwrap_or_default()
    }

    fn get_incoming_edges(&self) -> Vec<Self::EdgeAdapter> {
        let mut cache = self.inner.incoming_edge_adapters.borrow_mut();
        if cache.is_none() {
            let edges = ElkGraphUtil::all_incoming_edges(&self.inner.element);
            let adapters = edges.into_iter().map(ElkEdgeAdapter::new).collect();
            *cache = Some(adapters);
        }
        cache.as_ref().cloned().unwrap_or_default()
    }

    fn get_outgoing_edges(&self) -> Vec<Self::EdgeAdapter> {
        let mut cache = self.inner.outgoing_edge_adapters.borrow_mut();
        if cache.is_none() {
            let edges = ElkGraphUtil::all_outgoing_edges(&self.inner.element);
            let adapters = edges.into_iter().map(ElkEdgeAdapter::new).collect();
            *cache = Some(adapters);
        }
        cache.as_ref().cloned().unwrap_or_default()
    }

    fn sort_port_list(&self) {
        self.sort_port_list_by(|a, b| DEFAULT_PORTLIST_SORTER.compare(a, b));
    }

    fn sort_port_list_by<F>(&self, mut comparator: F)
    where
        F: FnMut(&Self::Port, &Self::Port) -> Ordering,
    {
        let constraints = with_node_properties_mut(&self.inner.element, |props| {
            props
                .get_property(CoreOptions::PORT_CONSTRAINTS)
                .unwrap_or(PortConstraints::Undefined)
        });
        if constraints.is_order_fixed() {
            let trace = *TRACE_CORE_PORT_SORT;
            let node_id = if trace {
                with_node_shape_mut(&self.inner.element, |shape| {
                    shape
                        .graph_element()
                        .identifier()
                        .unwrap_or("<no-node-id>")
                        .to_owned()
                })
            } else {
                String::new()
            };
            let before = if trace {
                let mut node_mut = self.inner.element.borrow_mut();
                node_mut
                    .ports()
                    .iter()
                    .map(|port| {
                        with_port_shape_mut(port, |shape| {
                            shape
                                .graph_element()
                                .identifier()
                                .unwrap_or("<no-port-id>")
                                .to_owned()
                        })
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                String::new()
            };
            let mut node_mut = self.inner.element.borrow_mut();
            node_mut.ports().sort_by(|a, b| comparator(a, b));
            if trace {
                let after = node_mut
                    .ports()
                    .iter()
                    .map(|port| {
                        with_port_shape_mut(port, |shape| {
                            shape
                                .graph_element()
                                .identifier()
                                .unwrap_or("<no-port-id>")
                                .to_owned()
                        })
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                eprintln!(
                    "rust-core-port-sort: node={} constraints={:?} before=[{}] after=[{}]",
                    node_id, constraints, before, after
                );
            }
        }
    }

    fn is_compound_node(&self) -> bool {
        let has_children = {
            let mut node_mut = self.inner.element.borrow_mut();
            !node_mut.children().is_empty()
        };
        let inside_self_loops = with_node_properties_mut(&self.inner.element, |props| {
            props
                .get_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
                .unwrap_or(false)
        });
        has_children || inside_self_loops
    }

    fn get_padding(&self) -> ElkPadding {
        with_node_properties_mut(&self.inner.element, |props| {
            props
                .get_property(CoreOptions::PADDING)
                .unwrap_or_else(ElkPadding::new)
        })
    }

    fn set_padding(&self, padding: ElkPadding) {
        with_node_properties_mut(&self.inner.element, |props| {
            props.set_property(CoreOptions::PADDING, Some(padding));
        });
    }

    fn get_margin(&self) -> ElkMargin {
        with_node_properties_mut(&self.inner.element, |props| {
            props
                .get_property(CoreOptions::MARGINS)
                .unwrap_or_else(ElkMargin::new)
        })
    }

    fn set_margin(&self, margin: ElkMargin) {
        with_node_properties_mut(&self.inner.element, |props| {
            props.set_property(CoreOptions::MARGINS, Some(margin));
        });
    }
}

impl GraphElementAdapter<ElkPortRef> for ElkPortAdapter {
    fn get_size(&self) -> KVector {
        with_port_shape_mut(&self.inner.element, |shape| {
            KVector::with_values(shape.width(), shape.height())
        })
    }

    fn set_size(&self, size: KVector) {
        with_port_shape_mut(&self.inner.element, |shape| {
            shape.set_width(size.x);
            shape.set_height(size.y);
        });
    }

    fn get_position(&self) -> KVector {
        with_port_shape_mut(&self.inner.element, |shape| {
            KVector::with_values(shape.x(), shape.y())
        })
    }

    fn set_position(&self, pos: KVector) {
        with_port_shape_mut(&self.inner.element, |shape| {
            shape.set_x(pos.x);
            shape.set_y(pos.y);
        });
    }

    fn get_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> Option<P> {
        with_port_properties_mut(&self.inner.element, |props| {
            get_property_with_offset(props, prop)
        })
    }

    fn has_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> bool {
        with_port_properties_mut(&self.inner.element, |props| props.has_property(prop))
    }

    fn get_volatile_id(&self) -> i32 {
        self.inner.volatile_id.get()
    }

    fn set_volatile_id(&self, volatile_id: i32) {
        self.inner.volatile_id.set(volatile_id);
    }
}

impl PortAdapter<ElkPortRef> for ElkPortAdapter {
    type Label = ElkLabelRef;
    type LabelAdapter = ElkLabelAdapter;
    type Edge = ElkEdgeRef;
    type EdgeAdapter = ElkEdgeAdapter;

    fn get_side(&self) -> PortSide {
        with_port_properties_mut(&self.inner.element, |props| {
            props
                .get_property(CoreOptions::PORT_SIDE)
                .unwrap_or(PortSide::Undefined)
        })
    }

    fn get_labels(&self) -> Vec<Self::LabelAdapter> {
        let mut cache = self.inner.label_adapters.borrow_mut();
        if cache.is_none() {
            let labels: Vec<ElkLabelRef> = {
                let mut port_mut = self.inner.element.borrow_mut();
                port_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect()
            };
            let adapters = labels.into_iter().map(ElkLabelAdapter::new).collect();
            *cache = Some(adapters);
        }
        cache.as_ref().cloned().unwrap_or_default()
    }

    fn get_margin(&self) -> ElkMargin {
        with_port_properties_mut(&self.inner.element, |props| {
            props
                .get_property(CoreOptions::MARGINS)
                .unwrap_or_else(ElkMargin::new)
        })
    }

    fn set_margin(&self, margin: ElkMargin) {
        with_port_properties_mut(&self.inner.element, |props| {
            props.set_property(CoreOptions::MARGINS, Some(margin));
        });
    }

    fn get_incoming_edges(&self) -> Vec<Self::EdgeAdapter> {
        let mut cache = self.inner.incoming_edge_adapters.borrow_mut();
        if cache.is_none() {
            let edges: Vec<ElkEdgeRef> = {
                let mut port_mut = self.inner.element.borrow_mut();
                port_mut.connectable().incoming_edges().iter().collect()
            };
            let adapters = edges.into_iter().map(ElkEdgeAdapter::new).collect();
            *cache = Some(adapters);
        }
        cache.as_ref().cloned().unwrap_or_default()
    }

    fn get_outgoing_edges(&self) -> Vec<Self::EdgeAdapter> {
        let mut cache = self.inner.outgoing_edge_adapters.borrow_mut();
        if cache.is_none() {
            let edges: Vec<ElkEdgeRef> = {
                let mut port_mut = self.inner.element.borrow_mut();
                port_mut.connectable().outgoing_edges().iter().collect()
            };
            let adapters = edges.into_iter().map(ElkEdgeAdapter::new).collect();
            *cache = Some(adapters);
        }
        cache.as_ref().cloned().unwrap_or_default()
    }

    fn has_compound_connections(&self) -> bool {
        let node = match self.inner.element.borrow().parent() {
            Some(node) => node,
            None => return false,
        };

        let outgoing_edges: Vec<ElkEdgeRef> = {
            let mut port_mut = self.inner.element.borrow_mut();
            port_mut.connectable().outgoing_edges().iter().collect()
        };
        for edge in outgoing_edges {
            let targets: Vec<ElkConnectableShapeRef> = {
                let edge_borrow = edge.borrow();
                edge_borrow.targets_ro().iter().cloned().collect()
            };
            for target in targets {
                if let Some(target_node) = ElkGraphUtil::connectable_shape_to_node(&target) {
                    if ElkGraphUtil::is_descendant(&target_node, &node) {
                        return true;
                    }
                    if Rc::ptr_eq(&target_node, &node) {
                        let inside_self_loops = with_edge_properties_mut(&edge, |props| {
                            props
                                .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
                                .unwrap_or(false)
                        });
                        if inside_self_loops {
                            return true;
                        }
                    }
                }
            }
        }

        let incoming_edges: Vec<ElkEdgeRef> = {
            let mut port_mut = self.inner.element.borrow_mut();
            port_mut.connectable().incoming_edges().iter().collect()
        };
        for edge in incoming_edges {
            let sources: Vec<ElkConnectableShapeRef> = {
                let edge_borrow = edge.borrow();
                edge_borrow.sources_ro().iter().cloned().collect()
            };
            for source in sources {
                if let Some(source_node) = ElkGraphUtil::connectable_shape_to_node(&source) {
                    if ElkGraphUtil::is_descendant(&source_node, &node) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

impl GraphElementAdapter<ElkLabelRef> for ElkLabelAdapter {
    fn get_size(&self) -> KVector {
        with_label_shape_mut(&self.inner.element, |shape| {
            KVector::with_values(shape.width(), shape.height())
        })
    }

    fn set_size(&self, size: KVector) {
        with_label_shape_mut(&self.inner.element, |shape| {
            shape.set_width(size.x);
            shape.set_height(size.y);
        });
    }

    fn get_position(&self) -> KVector {
        with_label_shape_mut(&self.inner.element, |shape| {
            KVector::with_values(shape.x(), shape.y())
        })
    }

    fn set_position(&self, pos: KVector) {
        with_label_shape_mut(&self.inner.element, |shape| {
            shape.set_x(pos.x);
            shape.set_y(pos.y);
        });
    }

    fn get_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> Option<P> {
        with_label_properties_mut(&self.inner.element, |props| {
            get_property_with_offset(props, prop)
        })
    }

    fn has_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> bool {
        with_label_properties_mut(&self.inner.element, |props| props.has_property(prop))
    }

    fn get_volatile_id(&self) -> i32 {
        self.inner.volatile_id.get()
    }

    fn set_volatile_id(&self, volatile_id: i32) {
        self.inner.volatile_id.set(volatile_id);
    }
}

impl LabelAdapter<ElkLabelRef> for ElkLabelAdapter {
    fn get_side(&self) -> LabelSide {
        with_label_properties_mut(&self.inner.element, |props| {
            props
                .get_property(LabelSide::LABEL_SIDE)
                .unwrap_or(LabelSide::Unknown)
        })
    }

    fn get_text(&self) -> String {
        self.inner.element.borrow().text().to_string()
    }
}

impl EdgeAdapter<ElkEdgeRef> for ElkEdgeAdapter {
    type Label = ElkLabelRef;
    type LabelAdapter = ElkLabelAdapter;

    fn get_labels(&self) -> Vec<Self::LabelAdapter> {
        let mut cache = self.inner.label_adapters.borrow_mut();
        if cache.is_none() {
            let labels: Vec<ElkLabelRef> = {
                let mut edge_mut = self.inner.element.borrow_mut();
                edge_mut.element().labels().iter().cloned().collect()
            };
            let adapters = labels.into_iter().map(ElkLabelAdapter::new).collect();
            *cache = Some(adapters);
        }
        cache.as_ref().cloned().unwrap_or_default()
    }
}

#[derive(Clone, Copy, Default)]
pub struct PortComparator;

impl PortComparator {
    pub fn compare(&self, port1: &ElkPortRef, port2: &ElkPortRef) -> Ordering {
        let side1 = with_port_properties_mut(port1, |props| {
            props
                .get_property(CoreOptions::PORT_SIDE)
                .unwrap_or(PortSide::Undefined)
        });
        let side2 = with_port_properties_mut(port2, |props| {
            props
                .get_property(CoreOptions::PORT_SIDE)
                .unwrap_or(PortSide::Undefined)
        });

        let ordinal_difference = (side1 as i32) - (side2 as i32);
        if ordinal_difference != 0 {
            return ordinal_difference.cmp(&0);
        }

        let index1 =
            with_port_properties_mut(port1, |props| props.get_property(CoreOptions::PORT_INDEX));
        let index2 =
            with_port_properties_mut(port2, |props| props.get_property(CoreOptions::PORT_INDEX));
        if let (Some(index1), Some(index2)) = (index1, index2) {
            let index_difference = index1 - index2;
            if index_difference != 0 {
                return index_difference.cmp(&0);
            }
        }

        match side1 {
            PortSide::North => {
                let x1 = with_port_shape_mut(port1, |shape| shape.x());
                let x2 = with_port_shape_mut(port2, |shape| shape.x());
                x1.partial_cmp(&x2).unwrap_or(Ordering::Equal)
            }
            PortSide::East => {
                let y1 = with_port_shape_mut(port1, |shape| shape.y());
                let y2 = with_port_shape_mut(port2, |shape| shape.y());
                y1.partial_cmp(&y2).unwrap_or(Ordering::Equal)
            }
            PortSide::South => {
                let x1 = with_port_shape_mut(port1, |shape| shape.x());
                let x2 = with_port_shape_mut(port2, |shape| shape.x());
                x2.partial_cmp(&x1).unwrap_or(Ordering::Equal)
            }
            PortSide::West => {
                let y1 = with_port_shape_mut(port1, |shape| shape.y());
                let y2 = with_port_shape_mut(port2, |shape| shape.y());
                y2.partial_cmp(&y1).unwrap_or(Ordering::Equal)
            }
            PortSide::Undefined => Ordering::Equal,
        }
    }
}

pub static DEFAULT_PORTLIST_SORTER: PortComparator = PortComparator;
