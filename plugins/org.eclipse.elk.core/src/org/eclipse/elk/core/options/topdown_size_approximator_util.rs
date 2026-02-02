use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;

use crate::org::eclipse::elk::core::options::CoreOptions;

pub struct TopdownSizeApproximatorUtil;

impl TopdownSizeApproximatorUtil {
    pub fn get_size_category_multiplier(original_graph: &ElkNodeRef) -> f64 {
        let parent = original_graph.borrow().parent();
        let this_graph_size = Self::get_graph_size(original_graph);
        let categories = with_node_properties_mut(original_graph, |props| {
            props
                .get_property(CoreOptions::TOPDOWN_SIZE_CATEGORIES)
                .unwrap_or(3)
        });

        if let Some(parent) = parent {
            let children: Vec<ElkNodeRef> = {
                let mut parent_mut = parent.borrow_mut();
                parent_mut.children().iter().cloned().collect()
            };
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
        let children: Vec<ElkNodeRef> = {
            let mut node_mut = original_graph.borrow_mut();
            node_mut.children().iter().cloned().collect()
        };

        let hierarchical_weight = with_node_properties_mut(original_graph, |props| {
            props
                .get_property(CoreOptions::TOPDOWN_SIZE_CATEGORIES_HIERARCHICAL_NODE_WEIGHT)
                .unwrap_or(4)
        });

        let mut sum = 0;
        for child in children {
            let has_children = {
                let mut child_mut = child.borrow_mut();
                !child_mut.children().is_empty()
            };
            if has_children {
                sum += hierarchical_weight;
            } else {
                sum += 1;
            }
        }
        sum
    }
}

fn with_node_properties_mut<R>(node: &ElkNodeRef, f: impl FnOnce(&mut MapPropertyHolder) -> R) -> R {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    f(props)
}
