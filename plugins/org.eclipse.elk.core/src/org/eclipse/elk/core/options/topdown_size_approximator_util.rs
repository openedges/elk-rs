use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::core::layout_arena_context::with_layout_arena;
use crate::org::eclipse::elk::core::options::CoreOptions;

pub struct TopdownSizeApproximatorUtil;

impl TopdownSizeApproximatorUtil {
    pub fn get_size_category_multiplier(original_graph: &ElkNodeRef) -> f64 {
        // Arena path: parent lookup without borrow
        let parent = with_layout_arena(|sync| {
            sync.node_id(original_graph)
                .and_then(|nid| sync.arena().node_parent[nid.idx()])
                .map(|pid| sync.node_ref(pid).clone())
        })
        .flatten()
        .or_else(|| original_graph.borrow().parent());

        let this_graph_size = Self::get_graph_size(original_graph);
        let categories = with_node_properties(original_graph, |props| {
            props
                .get_property(CoreOptions::TOPDOWN_SIZE_CATEGORIES)
                .unwrap_or(3)
        });

        if let Some(parent) = parent {
            // Arena path: children list without borrow
            let children = with_layout_arena(|sync| {
                sync.node_id(&parent).map(|nid| {
                    sync.arena().node_children[nid.idx()]
                        .iter()
                        .map(|&cid| sync.node_ref(cid).clone())
                        .collect::<Vec<_>>()
                })
            })
            .flatten()
            .unwrap_or_else(|| {
                let mut parent_mut = parent.borrow_mut();
                parent_mut.children().iter().cloned().collect()
            });

            let mut size_min_found = i32::MAX;
            let mut size_max_found = i32::MIN;
            for child in children {
                let size = Self::get_graph_size(&child);
                size_min_found = size_min_found.min(size);
                size_max_found = size_max_found.max(size);
            }

            let size_min = 1.0_f64;
            let mut size_max = 4.0_f64.powi(categories);
            if size_max_found as f64 > size_max {
                size_max = size_max_found as f64;
            }

            let x = (size_max.ln() - size_min.ln()) / categories as f64;
            let factor = x.exp();
            let mut cutoff = size_min * factor;
            for i in 0..categories {
                if (this_graph_size as f64) < cutoff {
                    return 2.0_f64.powi(i);
                }
                cutoff *= factor;
            }
            return 2.0_f64.powi(categories - 1);
        }

        1.0
    }

    pub fn get_graph_size(original_graph: &ElkNodeRef) -> i32 {
        // Arena path: children list without borrow
        let children = with_layout_arena(|sync| {
            sync.node_id(original_graph).map(|nid| {
                sync.arena().node_children[nid.idx()]
                    .iter()
                    .map(|&cid| sync.node_ref(cid).clone())
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .unwrap_or_else(|| {
            let mut node_mut = original_graph.borrow_mut();
            node_mut.children().iter().cloned().collect()
        });

        let hierarchical_weight = with_node_properties(original_graph, |props| {
            props
                .get_property(CoreOptions::TOPDOWN_SIZE_CATEGORIES_HIERARCHICAL_NODE_WEIGHT)
                .unwrap_or(4)
        });

        let mut sum = 0;
        for child in children {
            // Arena path: check if child has children without borrow
            let has_children = with_layout_arena(|sync| {
                sync.node_id(&child)
                    .map(|cid| !sync.arena().node_children[cid.idx()].is_empty())
            })
            .flatten()
            .unwrap_or_else(|| {
                let mut child_mut = child.borrow_mut();
                !child_mut.children().is_empty()
            });
            if has_children {
                sum += hierarchical_weight;
            } else {
                sum += 1;
            }
        }
        sum
    }
}

fn with_node_properties<R>(
    node: &ElkNodeRef,
    f: impl Fn(&MapPropertyHolder) -> R,
) -> R {
    // Arena path: read properties without borrow_mut
    if let Some(result) = with_layout_arena(|sync| {
        sync.node_id(node).map(|nid| f(&sync.arena().node_properties[nid.idx()]))
    })
    .flatten()
    {
        return result;
    }
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    f(props)
}
