use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;

use crate::org::eclipse::elk::alg::common::i_cost_function::ICostFunction;
use crate::org::eclipse::elk::alg::common::t_edge::TEdge;
use crate::org::eclipse::elk::alg::common::tree::Tree;
use crate::org::eclipse::elk::alg::common::utils::SVGImage;

pub struct NaiveMinST;

impl NaiveMinST {
    pub fn create_spanning_tree<C: ICostFunction>(
        t_edges: &HashSet<TEdge>,
        root: &KVector,
        cost_function: &C,
        debug_output_file: Option<&str>,
    ) -> Tree<KVector> {
        let mut weight: HashMap<TEdge, f64> = HashMap::new();
        for edge in t_edges {
            weight.insert(edge.clone(), cost_function.cost(edge));
        }

        let mut edge_list: Vec<TEdge> = t_edges.iter().cloned().collect();
        edge_list.sort_by(|e1, e2| {
            let w1 = weight.get(e1).copied().unwrap_or(0.0);
            let w2 = weight.get(e2).copied().unwrap_or(0.0);
            w1.partial_cmp(&w2).unwrap_or(Ordering::Equal)
        });
        let mut edges: Vec<TEdge> = edge_list;

        let mut min_st = Tree::new(*root);
        let mut tree_nodes: HashSet<KVector> = HashSet::new();
        tree_nodes.insert(*root);

        let mut svg = SVGImage::new(debug_output_file);
        svg.add_groups(&["e", "t"]);
        for edge in &edges {
            svg.g("e").add_line_with_attrs(
                edge.u.x,
                edge.u.y,
                edge.v.x,
                edge.v.y,
                "stroke=\"black\" stroke-width=\"1\"",
            );
            if let Some(weight) = weight.get(edge) {
                svg.g("t").add_element_str(&format!(
                    "<text x=\"{}\" y=\"{}\" fill=\"blue\" font-size=\"20px\">{:.2}</text>",
                    (edge.u.x + edge.v.x) / 2.0,
                    (edge.u.y + edge.v.y) / 2.0,
                    weight
                ));
            }
        }
        svg.isave();

        while !edges.is_empty() {
            let mut next_edge_index: Option<usize> = None;
            let mut next_node: Option<KVector> = None;
            let mut node_in_tree: Option<KVector> = None;

            for (idx, edge) in edges.iter().enumerate() {
                if tree_nodes.contains(&edge.u) && !tree_nodes.contains(&edge.v) {
                    next_edge_index = Some(idx);
                    next_node = Some(edge.v);
                    node_in_tree = Some(edge.u);
                    break;
                }
                if tree_nodes.contains(&edge.v) && !tree_nodes.contains(&edge.u) {
                    next_edge_index = Some(idx);
                    next_node = Some(edge.u);
                    node_in_tree = Some(edge.v);
                    break;
                }
            }

            let Some(edge_index) = next_edge_index else {
                break;
            };
            let Some(next_node) = next_node else {
                break;
            };
            let Some(node_in_tree) = node_in_tree else {
                break;
            };

            min_st.add_child_to(&node_in_tree, next_node);
            tree_nodes.insert(next_node);

            let used_edge = edges.remove(edge_index);
            svg.g("e").add_line_with_attrs(
                used_edge.u.x,
                used_edge.u.y,
                used_edge.v.x,
                used_edge.v.y,
                "stroke=\"red\" stroke-width=\"3\"",
            );
            svg.isave();
        }

        min_st
    }

    pub fn create_spanning_tree_without_debug<C: ICostFunction>(
        t_edges: &HashSet<TEdge>,
        root: &KVector,
        cost_function: &C,
    ) -> Tree<KVector> {
        Self::create_spanning_tree(t_edges, root, cost_function, None)
    }
}
