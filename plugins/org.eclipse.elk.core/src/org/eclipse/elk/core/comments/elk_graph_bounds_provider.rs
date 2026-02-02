use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkGraphElementRef, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

use crate::org::eclipse::elk::core::comments::i_bounds_provider::IBoundsProvider;
use crate::org::eclipse::elk::core::math::ElkRectangle;

pub trait BoundsProviderShape {
    fn bounds(&self) -> Option<ElkRectangle>;
}

impl BoundsProviderShape for ElkNodeRef {
    fn bounds(&self) -> Option<ElkRectangle> {
        let mut node_mut = self.borrow_mut();
        let shape = node_mut.connectable().shape();
        Some(ElkRectangle::with_values(
            shape.x(),
            shape.y(),
            shape.width(),
            shape.height(),
        ))
    }
}

impl BoundsProviderShape for ElkPortRef {
    fn bounds(&self) -> Option<ElkRectangle> {
        let mut port_mut = self.borrow_mut();
        let shape = port_mut.connectable().shape();
        Some(ElkRectangle::with_values(
            shape.x(),
            shape.y(),
            shape.width(),
            shape.height(),
        ))
    }
}

impl BoundsProviderShape for ElkLabelRef {
    fn bounds(&self) -> Option<ElkRectangle> {
        let mut label_mut = self.borrow_mut();
        let shape = label_mut.shape();
        Some(ElkRectangle::with_values(
            shape.x(),
            shape.y(),
            shape.width(),
            shape.height(),
        ))
    }
}

impl BoundsProviderShape for ElkGraphElementRef {
    fn bounds(&self) -> Option<ElkRectangle> {
        match self {
            ElkGraphElementRef::Node(node) => node.bounds(),
            ElkGraphElementRef::Port(port) => port.bounds(),
            ElkGraphElementRef::Label(label) => label.bounds(),
            ElkGraphElementRef::Edge(_edge) => None,
        }
    }
}

pub struct ElkGraphBoundsProvider;

impl<S: BoundsProviderShape + 'static> IBoundsProvider<S, S> for ElkGraphBoundsProvider {
    fn bounds_for_comment(&self, comment: &S) -> Option<ElkRectangle> {
        comment.bounds()
    }

    fn bounds_for_target(&self, target: &S) -> Option<ElkRectangle> {
        target.bounds()
    }
}
