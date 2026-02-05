use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::options::RectPackingOptions;

pub struct InteractiveNodeReorderer;

impl ILayoutProcessor<ElkNodeRef> for InteractiveNodeReorderer {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Interactive Node Reorderer", 1.0);
        let mut rectangles = collect_children(graph);
        let mut fixed_nodes = Vec::new();

        for rect in &rectangles {
            let has_property = {
                let mut rect_mut = rect.borrow_mut();
                rect_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .has_property(RectPackingOptions::DESIRED_POSITION)
            };
            if has_property {
                fixed_nodes.push(rect.clone());
            }
        }

        rectangles.retain(|node| !fixed_nodes.iter().any(|f| std::rc::Rc::ptr_eq(f, node)));

        fixed_nodes.sort_by(|a, b| {
            let position_a = get_desired_position(a);
            let position_b = get_desired_position(b);
            if position_a == position_b {
                std::cmp::Ordering::Less
            } else {
                position_a.cmp(&position_b)
            }
        });

        for node in fixed_nodes {
            let position = get_desired_position(&node);
            let insert_pos = position.min(rectangles.len() as i32).max(0) as usize;
            rectangles.insert(insert_pos, node);
        }

        for (index, node) in rectangles.iter().enumerate() {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .set_property(RectPackingOptions::CURRENT_POSITION, Some(index as i32));
        }

        reorder_children(graph, rectangles);
        progress_monitor.done();
    }
}

fn collect_children(graph: &ElkNodeRef) -> Vec<ElkNodeRef> {
    let mut graph_mut = graph.borrow_mut();
    graph_mut.children().iter().cloned().collect()
}

fn reorder_children(graph: &ElkNodeRef, new_order: Vec<ElkNodeRef>) {
    let mut graph_mut = graph.borrow_mut();
    let children = graph_mut.children();
    while children.len() > 0 {
        children.remove_at(0);
    }
    for child in new_order {
        children.add(child);
    }
}

fn get_desired_position(node: &ElkNodeRef) -> i32 {
    let mut node_mut = node.borrow_mut();
    node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .get_property(RectPackingOptions::DESIRED_POSITION)
        .unwrap_or(-1)
}
