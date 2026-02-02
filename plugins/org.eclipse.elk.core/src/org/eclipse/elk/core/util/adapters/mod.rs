use std::cmp::Ordering;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector};
use crate::org::eclipse::elk::core::options::{LabelSide, PortSide};

pub mod elk_graph_adapters;

pub use elk_graph_adapters::{
    ElkEdgeAdapter, ElkGraphAdapter, ElkGraphAdapters, ElkLabelAdapter, ElkNodeAdapter,
    ElkPortAdapter, PortComparator, DEFAULT_PORTLIST_SORTER,
};

pub trait GraphElementAdapter<T> {
    fn get_size(&self) -> KVector;
    fn set_size(&self, size: KVector);
    fn get_position(&self) -> KVector;
    fn set_position(&self, pos: KVector);
    fn get_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> Option<P>;
    fn has_property<P: Clone + Send + Sync + 'static>(&self, prop: &Property<P>) -> bool;
    fn get_volatile_id(&self) -> i32;
    fn set_volatile_id(&self, volatile_id: i32);
}

pub trait GraphAdapter<T>: GraphElementAdapter<T> {
    type Node;
    type NodeAdapter: NodeAdapter<Self::Node>;

    fn get_nodes(&self) -> Vec<Self::NodeAdapter>;
}

pub trait NodeAdapter<T>: GraphElementAdapter<T> {
    type Graph;
    type Label;
    type LabelAdapter: LabelAdapter<Self::Label>;
    type Port;
    type PortAdapter: PortAdapter<Self::Port>;
    type Edge;
    type EdgeAdapter: EdgeAdapter<Self::Edge>;

    fn get_graph(&self) -> Option<Self::Graph>;
    fn get_labels(&self) -> Vec<Self::LabelAdapter>;
    fn get_ports(&self) -> Vec<Self::PortAdapter>;
    fn get_incoming_edges(&self) -> Vec<Self::EdgeAdapter>;
    fn get_outgoing_edges(&self) -> Vec<Self::EdgeAdapter>;
    fn sort_port_list(&self);
    fn sort_port_list_by<F>(&self, comparator: F)
    where
        F: FnMut(&Self::Port, &Self::Port) -> Ordering;
    fn is_compound_node(&self) -> bool;
    fn get_padding(&self) -> ElkPadding;
    fn set_padding(&self, padding: ElkPadding);
    fn get_margin(&self) -> ElkMargin;
    fn set_margin(&self, margin: ElkMargin);
}

pub trait PortAdapter<T>: GraphElementAdapter<T> {
    type Label;
    type LabelAdapter: LabelAdapter<Self::Label>;
    type Edge;
    type EdgeAdapter: EdgeAdapter<Self::Edge>;

    fn get_side(&self) -> PortSide;
    fn get_labels(&self) -> Vec<Self::LabelAdapter>;
    fn get_margin(&self) -> ElkMargin;
    fn set_margin(&self, margin: ElkMargin);
    fn get_incoming_edges(&self) -> Vec<Self::EdgeAdapter>;
    fn get_outgoing_edges(&self) -> Vec<Self::EdgeAdapter>;
    fn has_compound_connections(&self) -> bool;
}

pub trait LabelAdapter<T>: GraphElementAdapter<T> {
    fn get_side(&self) -> LabelSide;
    fn get_text(&self) -> String;
}

pub trait EdgeAdapter<T> {
    type Label;
    type LabelAdapter: LabelAdapter<Self::Label>;

    fn get_labels(&self) -> Vec<Self::LabelAdapter>;
}
