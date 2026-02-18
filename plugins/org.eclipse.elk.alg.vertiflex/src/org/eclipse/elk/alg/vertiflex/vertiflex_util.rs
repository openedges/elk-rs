use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub struct VertiFlexUtil;

impl VertiFlexUtil {
    pub fn find_root(graph: &ElkNodeRef) -> Option<ElkNodeRef> {
        let children: Vec<ElkNodeRef> = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().iter().cloned().collect()
        };

        children
            .into_iter()
            .find(|child| ElkGraphUtil::all_incoming_edges(child).is_empty())
    }
}
