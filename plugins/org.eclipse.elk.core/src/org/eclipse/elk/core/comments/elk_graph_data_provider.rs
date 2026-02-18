use std::rc::Rc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkConnectableShapeRef, ElkNodeRef};

use crate::org::eclipse::elk::core::comments::i_data_provider::IDataProvider;
use crate::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;

pub struct ElkGraphDataProvider {
    graph: ElkNodeRef,
}

impl ElkGraphDataProvider {
    pub fn new(graph: ElkNodeRef) -> Self {
        ElkGraphDataProvider { graph }
    }
}

impl IDataProvider<ElkNodeRef, ElkNodeRef> for ElkGraphDataProvider {
    fn provide_comments(&self) -> Vec<ElkNodeRef> {
        let children = node_children(&self.graph);
        children.into_iter().filter(is_comment).collect()
    }

    fn provide_targets(&self) -> Vec<ElkNodeRef> {
        let children = node_children(&self.graph);
        children
            .into_iter()
            .filter(|node| !is_comment(node))
            .collect()
    }

    fn provide_sub_hierarchies(&self) -> Vec<Rc<dyn IDataProvider<ElkNodeRef, ElkNodeRef>>> {
        let children = node_children(&self.graph);
        children
            .into_iter()
            .filter(|node| node.borrow().is_hierarchical())
            .map(|node| {
                Rc::new(ElkGraphDataProvider::new(node))
                    as Rc<dyn IDataProvider<ElkNodeRef, ElkNodeRef>>
            })
            .collect()
    }

    fn attach(&self, comment: &ElkNodeRef, target: &ElkNodeRef) {
        ElkGraphUtil::create_simple_edge(
            ElkConnectableShapeRef::Node(comment.clone()),
            ElkConnectableShapeRef::Node(target.clone()),
        );
    }
}

fn is_comment(node: &ElkNodeRef) -> bool {
    with_node_properties_mut(node, |props| {
        props
            .get_property(CoreOptions::COMMENT_BOX)
            .unwrap_or(false)
    })
}

fn node_children(node: &ElkNodeRef) -> Vec<ElkNodeRef> {
    let mut node_mut = node.borrow_mut();
    node_mut.children().iter().cloned().collect()
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
