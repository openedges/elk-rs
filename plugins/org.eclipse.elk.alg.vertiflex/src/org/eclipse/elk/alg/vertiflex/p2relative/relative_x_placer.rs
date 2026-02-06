use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::Pair;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::vertiflex::edge_routing_strategy::EdgeRoutingStrategy;
use crate::org::eclipse::elk::alg::vertiflex::internal_properties::InternalProperties;
use crate::org::eclipse::elk::alg::vertiflex::options::VertiFlexOptions;
use crate::org::eclipse::elk::alg::vertiflex::p2relative::node_comparator::NodeComparator;
use crate::org::eclipse::elk::alg::vertiflex::p2relative::outline_node::OutlineNode;
use crate::org::eclipse::elk::alg::vertiflex::vertiflex_layout_phases::VertiFlexLayoutPhases;
use crate::org::eclipse::elk::alg::vertiflex::vertiflex_util::VertiFlexUtil;

pub struct RelativeXPlacer {
    spacing_node_node: f64,
    consider_node_model_order: bool,
}

impl RelativeXPlacer {
    pub fn new() -> Self {
        RelativeXPlacer {
            spacing_node_node: 0.0,
            consider_node_model_order: false,
        }
    }

    fn outline_distance(&self, outline1: &OutlineNode, outline2: &OutlineNode) -> f64 {
        let changed_outline1 = OutlineNode::new(
            outline1.relative_x(),
            MINIMAL_Y,
            Some(OutlineNode::new(0.0, outline1.absolute_y(), outline1.next_cloned())),
        );
        let changed_outline2 = OutlineNode::new(
            outline2.relative_x(),
            MINIMAL_Y,
            Some(OutlineNode::new(0.0, outline2.absolute_y(), outline2.next_cloned())),
        );

        let mut dist = changed_outline1.relative_x() - changed_outline2.relative_x();

        let mut o1 = Some(&changed_outline1);
        let mut o2 = Some(&changed_outline2);
        let mut x1 = changed_outline1.relative_x();
        let mut x2 = changed_outline2.relative_x();

        while let (Some(o1_node), Some(o2_node)) = (o1, o2) {
            if o2_node.is_last() {
                break;
            }
            let o2_next = o2_node.next().expect("next");
            if o2_next.absolute_y() > o1_node.absolute_y() {
                let delta_x = o2_next.relative_x();
                let delta_y = o2_next.absolute_y() - o2_node.absolute_y();
                let newdist = x1
                    - x2
                    - ((o1_node.absolute_y() - o2_node.absolute_y()) * delta_x) / delta_y;
                if newdist > dist {
                    dist = newdist;
                }
                o1 = o1_node.next();
                if let Some(next) = o1 {
                    x1 += next.relative_x();
                }
            } else {
                o2 = o2_node.next();
                if let Some(next) = o2 {
                    x2 += next.relative_x();
                }
            }
        }

        o1 = Some(&changed_outline1);
        o2 = Some(&changed_outline2);
        x1 = changed_outline1.relative_x();
        x2 = changed_outline2.relative_x();
        while let (Some(o1_node), Some(o2_node)) = (o1, o2) {
            if o1_node.is_last() {
                break;
            }
            let o1_next = o1_node.next().expect("next");
            if o1_next.absolute_y() > o2_node.absolute_y() {
                let delta_x = o1_next.relative_x();
                let delta_y = o1_next.absolute_y() - o1_node.absolute_y();
                let newdist =
                    x1 - x2 + ((o2_node.absolute_y() - o1_node.absolute_y()) * delta_x) / delta_y;
                if newdist > dist {
                    dist = newdist;
                }
                o2 = o2_node.next();
                if let Some(next) = o2 {
                    x2 += next.relative_x();
                }
            } else {
                o1 = o1_node.next();
                if let Some(next) = o1 {
                    x1 += next.relative_x();
                }
            }
        }

        dist
    }

    fn recursive_straightline_placement(&self, graph: &ElkNodeRef) {
        self.make_simple_outlines(graph);

        let outgoing = ElkGraphUtil::all_outgoing_edges(graph);
        if outgoing.is_empty() {
            return;
        }

        let mut children = Vec::new();
        for edge in outgoing {
            if let Some(child) = edge_target_node(&edge) {
                self.recursive_straightline_placement(&child);
                children.push(child);
            }
        }

        self.sort_subtrees(&mut children);

        for i in 0..children.len().saturating_sub(1) {
            self.bundle_children(&children[0], &children[i], &children[i + 1]);
        }

        let mut pos = 0;
        let mut max_depth = 0.0;
        let mut max_depth_start_pos = 0;
        while pos < children.len() && node_y(&children[pos]) >= max_depth {
            let child_y = node_y(&children[pos]);
            if child_y > max_depth {
                max_depth_start_pos = pos;
                max_depth = child_y;
            }
            pos += 1;
        }

        let mut move_root = 0.0;
        if pos > 0 {
            let child_start = &children[max_depth_start_pos];
            let child_end = &children[pos - 1];
            move_root = (node_x(child_start) + node_x(child_end) + node_width(child_end)) / 2.0
                - node_x(graph)
                - node_width(graph) / 2.0;
        }

        if !self.consider_node_model_order {
            let mut better_move_root =
                (node_x(&children[0]) + node_x(children.last().unwrap())
                    + node_width(children.last().unwrap())
                    - node_width(graph))
                    / 2.0
                    - node_x(graph);
            let mut new_move_root;

            if better_move_root < move_root {
                for i in 0..max_depth_start_pos {
                    for j in (i + 1)..=max_depth_start_pos {
                        let right_outline =
                            node_get_property(&children[i], InternalProperties::RIGHT_OUTLINE)
                                .expect("right outline");
                        let mut right_outline_x = node_x(&children[i]) + right_outline.relative_x();
                        let pos_x = node_x(&children[j]) + node_width(&children[j]) / 2.0;
                        let mut outline_iter = Some(right_outline);
                        while let Some(outline) = outline_iter {
                            if outline.absolute_y() >= max_depth {
                                break;
                            }
                            new_move_root = pos_x
                                - node_width(graph) / 2.0
                                + (pos_x - right_outline_x)
                                    * ((node_y(graph) + node_height(graph)) - max_depth)
                                    / (max_depth - outline.absolute_y());
                            if new_move_root > better_move_root {
                                better_move_root = new_move_root;
                            }

                            outline_iter = outline.next_cloned();
                            if let Some(next) = outline_iter.as_ref() {
                                right_outline_x += next.relative_x();
                            }
                        }
                    }
                }
                move_root = better_move_root;
            }

            if better_move_root > move_root {
                for i in pos..children.len() {
                    for j in (pos.saturating_sub(1))..i {
                        let left_outline =
                            node_get_property(&children[i], InternalProperties::LEFT_OUTLINE)
                                .expect("left outline");
                        let mut left_outline_x = node_x(&children[i]) + left_outline.relative_x();
                        let pos_x = node_x(&children[j]) + node_width(&children[j]) / 2.0;
                        let mut outline_iter = Some(left_outline);
                        while let Some(outline) = outline_iter {
                            if outline.absolute_y() >= max_depth {
                                break;
                            }
                            new_move_root = pos_x
                                - node_width(graph) / 2.0
                                + (pos_x - left_outline_x)
                                    * ((node_y(graph) + node_height(graph)) - max_depth)
                                    / (max_depth - outline.absolute_y());
                            if new_move_root < better_move_root {
                                better_move_root = new_move_root;
                            }

                            outline_iter = outline.next_cloned();
                            if let Some(next) = outline_iter.as_ref() {
                                left_outline_x += next.relative_x();
                            }
                        }
                    }
                }
                move_root = better_move_root;
            }
        }

        for child in &children {
            node_set_x(child, node_x(child) - move_root);
        }

        let mut graph_left_outline =
            node_get_property(graph, InternalProperties::LEFT_OUTLINE).expect("left outline");
        let left_child_outline =
            node_get_property(&children[0], InternalProperties::LEFT_OUTLINE).expect("left outline");

        let new_x = node_x(&children[0]) + left_child_outline.relative_x() - graph_left_outline.relative_x();
        let new_outline = OutlineNode::new(new_x, left_child_outline.absolute_y(), left_child_outline.next_cloned());
        set_outline_third_next(&mut graph_left_outline, new_outline);
        node_set_property(graph, InternalProperties::LEFT_OUTLINE, Some(graph_left_outline));

        let mut graph_right_outline =
            node_get_property(graph, InternalProperties::RIGHT_OUTLINE).expect("right outline");
        let right_child_outline = node_get_property(children.last().unwrap(), InternalProperties::RIGHT_OUTLINE)
            .expect("right outline");

        let new_x = node_x(children.last().unwrap())
            + right_child_outline.relative_x()
            - graph_right_outline.relative_x();
        let new_outline = OutlineNode::new(new_x, right_child_outline.absolute_y(), right_child_outline.next_cloned());
        set_outline_third_next(&mut graph_right_outline, new_outline);
        node_set_property(graph, InternalProperties::RIGHT_OUTLINE, Some(graph_right_outline));

        let mut outline_max_depth =
            node_get_property(graph, InternalProperties::OUTLINE_MAX_DEPTH).unwrap_or(0.0);
        let mut min_x = node_get_property(graph, InternalProperties::MIN_X).unwrap_or(0.0);
        let mut max_x = node_get_property(graph, InternalProperties::MAX_X).unwrap_or(0.0);
        for child in &children {
            let child_depth =
                node_get_property(child, InternalProperties::OUTLINE_MAX_DEPTH).unwrap_or(0.0);
            outline_max_depth = outline_max_depth.max(child_depth);
            let child_min_x = node_get_property(child, InternalProperties::MIN_X).unwrap_or(0.0);
            let child_max_x = node_get_property(child, InternalProperties::MAX_X).unwrap_or(0.0);
            min_x = min_x.min(node_x(child) + child_min_x);
            max_x = max_x.max(node_x(child) + child_max_x);
        }
        node_set_property(graph, InternalProperties::OUTLINE_MAX_DEPTH, Some(outline_max_depth));
        node_set_property(graph, InternalProperties::MIN_X, Some(min_x));
        node_set_property(graph, InternalProperties::MAX_X, Some(max_x));
        node_set_property(graph, InternalProperties::MAX_Y, Some(outline_max_depth));
    }

    fn recursive_bentline_placement(&self, graph: &ElkNodeRef) {
        self.make_simple_outlines(graph);

        let outgoing = ElkGraphUtil::all_outgoing_edges(graph);
        if outgoing.is_empty() {
            return;
        }

        let mut children = Vec::new();
        for edge in outgoing {
            if let Some(child) = edge_target_node(&edge) {
                self.recursive_bentline_placement(&child);
                children.push(child);
            }
        }

        for i in 0..children.len().saturating_sub(1) {
            self.bundle_children(&children[0], &children[i], &children[i + 1]);
        }

        let move_root = (node_x(&children[0])
            + node_width(&children[0]) / 2.0
            + node_x(children.last().unwrap())
            + node_width(children.last().unwrap()) / 2.0
            - node_width(graph))
            / 2.0
            - node_x(graph);

        for child in &children {
            node_set_x(child, node_x(child) - move_root);
            let bend_height =
                node_get_property(child, InternalProperties::LEFT_OUTLINE)
                    .expect("left outline")
                    .absolute_y();
            node_set_property(child, InternalProperties::EDGE_BEND_HEIGHT, Some(bend_height));
        }

        let children_size = children.len();
        let mut i = 0;
        while i < children_size - 1
            && node_x(&children[i])
                + node_width(&children[i])
                + node_get_property(&children[i], CoreOptions::MARGINS)
                    .unwrap_or_default()
                    .right
                - node_width(graph) / 2.0
                <= 0.0
        {
            i += 1;
        }

        let mut global_bend_height =
            node_get_property(&children[i], InternalProperties::EDGE_BEND_HEIGHT).unwrap_or(0.0);
        for child in &children {
            let child_height =
                node_get_property(child, InternalProperties::EDGE_BEND_HEIGHT).unwrap_or(0.0);
            if global_bend_height < child_height {
                node_set_property(child, InternalProperties::EDGE_BEND_HEIGHT, Some(global_bend_height));
            } else {
                global_bend_height = child_height;
            }
        }

        i = children_size - 1;
        while i > 0
            && node_x(&children[i])
                - node_get_property(&children[i], CoreOptions::MARGINS)
                    .unwrap_or_default()
                    .left
                - node_width(graph) / 2.0
                >= 0.0
        {
            i -= 1;
        }

        if i < children_size {
            for a in (0..=i).rev() {
                let child_height =
                    node_get_property(&children[a], InternalProperties::EDGE_BEND_HEIGHT).unwrap_or(0.0);
                if global_bend_height < child_height {
                    node_set_property(&children[a], InternalProperties::EDGE_BEND_HEIGHT, Some(global_bend_height));
                } else {
                    global_bend_height = child_height;
                }
            }
        }

        let left_child_outline =
            node_get_property(&children[0], InternalProperties::LEFT_OUTLINE).expect("left outline");
        let mut graph_left_outline =
            node_get_property(graph, InternalProperties::LEFT_OUTLINE).expect("left outline");
        let new_x = node_x(&children[0]) + left_child_outline.relative_x() - graph_left_outline.relative_x();
        let new_outline_part = OutlineNode::new(0.0, left_child_outline.absolute_y(), left_child_outline.next_cloned());
        let new_outline = OutlineNode::new(
            new_x,
            node_get_property(&children[0], InternalProperties::EDGE_BEND_HEIGHT).unwrap_or(0.0),
            Some(new_outline_part),
        );
        set_outline_third_next(&mut graph_left_outline, new_outline);
        node_set_property(graph, InternalProperties::LEFT_OUTLINE, Some(graph_left_outline));

        let right_child_outline = node_get_property(children.last().unwrap(), InternalProperties::RIGHT_OUTLINE)
            .expect("right outline");
        let mut graph_right_outline =
            node_get_property(graph, InternalProperties::RIGHT_OUTLINE).expect("right outline");
        let new_x = node_x(children.last().unwrap()) + right_child_outline.relative_x()
            - graph_right_outline.relative_x();
        let new_outline_part = OutlineNode::new(0.0, right_child_outline.absolute_y(), right_child_outline.next_cloned());
        let new_outline = OutlineNode::new(
            new_x,
            node_get_property(children.last().unwrap(), InternalProperties::EDGE_BEND_HEIGHT).unwrap_or(0.0),
            Some(new_outline_part),
        );
        set_outline_third_next(&mut graph_right_outline, new_outline);
        node_set_property(graph, InternalProperties::RIGHT_OUTLINE, Some(graph_right_outline));

        let mut outline_max_depth =
            node_get_property(graph, InternalProperties::OUTLINE_MAX_DEPTH).unwrap_or(0.0);
        let mut min_x = node_get_property(graph, InternalProperties::MIN_X).unwrap_or(0.0);
        let mut max_x = node_get_property(graph, InternalProperties::MAX_X).unwrap_or(0.0);
        for child in &children {
            let child_depth =
                node_get_property(child, InternalProperties::OUTLINE_MAX_DEPTH).unwrap_or(0.0);
            outline_max_depth = outline_max_depth.max(child_depth);
            let child_min_x = node_get_property(child, InternalProperties::MIN_X).unwrap_or(0.0);
            let child_max_x = node_get_property(child, InternalProperties::MAX_X).unwrap_or(0.0);
            min_x = min_x.min(node_x(child) + child_min_x);
            max_x = max_x.max(node_x(child) + child_max_x);
        }
        node_set_property(graph, InternalProperties::OUTLINE_MAX_DEPTH, Some(outline_max_depth));
        node_set_property(graph, InternalProperties::MIN_X, Some(min_x));
        node_set_property(graph, InternalProperties::MAX_X, Some(max_x));
        node_set_property(graph, InternalProperties::MAX_Y, Some(outline_max_depth));
    }

    fn sort_subtrees(&self, children: &mut [ElkNodeRef]) {
        let comparator = NodeComparator::new(false);
        children.sort_by(|a, b| comparator.compare(a, b));

        let mut left = Vec::new();
        let mut right = Vec::new();

        if self.consider_node_model_order {
            self.split_nodes_with_model_order(children, &mut left, &mut right);
        } else if !children.is_empty() {
            left.push(children[children.len() - 1].clone());
            let mut width_left = 0.0;
            let mut width_right = 0.0;
            for i in 1..children.len() {
                let idx = children.len() - 1 - i;
                if width_left <= width_right {
                    left.push(children[idx].clone());
                    width_left += node_width(&children[idx]);
                } else {
                    right.push(children[idx].clone());
                    width_right += node_width(&children[idx]);
                }
            }
            left.reverse();
        }

        let comparator = NodeComparator::new(true);
        right.sort_by(|a, b| comparator.compare(a, b));
        left.extend(right);
        for (idx, node) in left.into_iter().enumerate() {
            children[idx] = node;
        }
    }

    fn split_nodes_with_model_order(
        &self,
        original: &[ElkNodeRef],
        left: &mut Vec<ElkNodeRef>,
        right: &mut Vec<ElkNodeRef>,
    ) {
        if original.is_empty() {
            return;
        }
        if original.len() == 1 {
            left.push(original[0].clone());
        }
        if original.len() == 2 {
            let first = &original[0];
            let second = &original[1];
            let first_order =
                node_get_property(first, InternalProperties::NODE_MODEL_ORDER).unwrap_or(0);
            let second_order =
                node_get_property(second, InternalProperties::NODE_MODEL_ORDER).unwrap_or(0);
            if first_order > second_order {
                left.push(second.clone());
                right.push(first.clone());
            } else {
                left.push(first.clone());
                right.push(second.clone());
            }
        }

        let mut current_group = Vec::new();
        let mut width_left_right = Pair::create();
        width_left_right.set_first(0.0);
        width_left_right.set_second(0.0);
        let mut current = original[0].clone();
        current_group.push(current.clone());
        for i in 1..original.len() {
            let next = original[i].clone();
            if node_y(&current) == node_y(&next) {
                current_group.push(next.clone());
            } else {
                let final_index_of_left = self.split_group(&current_group, &mut width_left_right);
                left.extend(current_group.iter().take(final_index_of_left + 1).cloned());
                right.extend(current_group.iter().skip(final_index_of_left + 1).cloned());
                current_group.clear();
                current_group.push(next.clone());
            }
            current = next;

            if i == original.len() - 1 {
                let final_index_of_left = self.split_group(&current_group, &mut width_left_right);
                left.extend(current_group.iter().take(final_index_of_left + 1).cloned());
                right.extend(current_group.iter().skip(final_index_of_left + 1).cloned());
            }
        }
    }

    fn split_group(
        &self,
        group: &[ElkNodeRef],
        width_left_right: &mut Pair<f64, f64>,
    ) -> usize {
        if group.len() == 1 {
            return 0;
        }

        let width_left = *width_left_right.first();
        let width_right = *width_left_right.second();
        let mut total_new_width = 0.0;
        for node in group {
            total_new_width += node_width(node);
        }

        let desired_left = (total_new_width - width_left + width_right) / 2.0;

        let mut i = 0;
        let mut new_left_width = 0.0;
        while desired_left < new_left_width && i < group.len() {
            new_left_width += node_width(&group[i]);
            i += 1;
        }

        let exceed = new_left_width - desired_left;
        let under = desired_left - (new_left_width - node_width(&group[i.saturating_sub(1)]));

        let result_index = if exceed > under { i } else { i.saturating_sub(1) };

        width_left_right.set_first(width_left + exceed);
        let mut new_right_width = 0.0;
        for node in group.iter().skip(result_index + 1) {
            new_right_width += node_width(node);
        }
        width_left_right.set_second(width_right + new_right_width);

        result_index
    }

    fn make_simple_outlines(&self, graph: &ElkNodeRef) {
        let margins = node_get_property(graph, CoreOptions::MARGINS)
            .unwrap_or_default();

        let end_part = OutlineNode::new(
            0.0,
            node_y(graph) + node_height(graph) + margins.bottom + self.spacing_node_node / 2.0,
            Some(OutlineNode::new(
                node_width(graph) / 2.0,
                node_y(graph) + node_height(graph) + margins.bottom + self.spacing_node_node / 2.0,
                None,
            )),
        );

        let left_outline = OutlineNode::new(
            (-margins.left - self.spacing_node_node / 2.0) + node_width(graph) / 2.0,
            node_y(graph) - margins.top - self.spacing_node_node / 2.0,
            Some(OutlineNode::new(
                -node_width(graph) / 2.0,
                node_y(graph) - margins.top,
                Some(end_part),
            )),
        );

        let end_part = OutlineNode::new(
            0.0,
            node_y(graph) + node_height(graph) + margins.bottom,
            Some(OutlineNode::new(
                -node_width(graph) / 2.0,
                node_y(graph) + node_height(graph) + margins.bottom + self.spacing_node_node / 2.0,
                None,
            )),
        );

        let right_outline = OutlineNode::new(
            node_width(graph) / 2.0 + margins.right + self.spacing_node_node / 2.0,
            node_y(graph) - margins.top,
            Some(OutlineNode::new(
                node_width(graph) / 2.0,
                node_y(graph) - margins.top - self.spacing_node_node / 2.0,
                Some(end_part),
            )),
        );

        node_set_property(graph, InternalProperties::LEFT_OUTLINE, Some(left_outline));
        node_set_property(graph, InternalProperties::RIGHT_OUTLINE, Some(right_outline));
        node_set_property(graph, InternalProperties::MIN_X, Some(node_x(graph) - margins.left));
        node_set_property(
            graph,
            InternalProperties::MAX_X,
            Some(node_x(graph) + margins.right + node_width(graph)),
        );
        node_set_property(graph, InternalProperties::MIN_Y, Some(node_y(graph) - margins.top));
        node_set_property(
            graph,
            InternalProperties::MAX_Y,
            Some(node_y(graph) + margins.bottom + node_height(graph)),
        );

        let outline_max_depth = node_get_property(graph, InternalProperties::LEFT_OUTLINE)
            .expect("left outline")
            .next()
            .and_then(|node| node.next())
            .map(|node| node.absolute_y())
            .unwrap_or(0.0);
        node_set_property(graph, InternalProperties::OUTLINE_MAX_DEPTH, Some(outline_max_depth));
    }

    fn bundle_children(&self, left_subtree: &ElkNodeRef, a: &ElkNodeRef, b: &ElkNodeRef) {
        let dist = self.outline_distance(
            &node_get_property(a, InternalProperties::RIGHT_OUTLINE).expect("right outline"),
            &node_get_property(b, InternalProperties::LEFT_OUTLINE).expect("left outline"),
        );
        node_set_x(b, node_x(a) + dist);

        let left_max_depth =
            node_get_property(left_subtree, InternalProperties::OUTLINE_MAX_DEPTH).unwrap_or(0.0);
        let b_max_depth = node_get_property(b, InternalProperties::OUTLINE_MAX_DEPTH).unwrap_or(0.0);
        if left_max_depth < b_max_depth {
            let mut left_outline = node_get_property(left_subtree, InternalProperties::LEFT_OUTLINE)
                .expect("left outline");
            let (l_abs_x, last_l_abs_y) =
                outline_abs_x_and_last_y(&left_outline, node_x(left_subtree));

            let mut b_iterator = {
                let b_left = node_get_property(b, InternalProperties::LEFT_OUTLINE)
                    .expect("left outline");
                OutlineNode::new(b_left.relative_x(), MINIMAL_Y, b_left.next_cloned())
            };
            let mut r_abs_x = b_iterator.relative_x() + node_x(b);
            while b_iterator
                .next()
                .map(|node| node.absolute_y() <= last_l_abs_y)
                .unwrap_or(false)
            {
                let next = b_iterator.next_cloned().expect("next");
                r_abs_x += next.relative_x();
                b_iterator = next;
            }

            let b_next = b_iterator.next().expect("next");
            let delta_x = b_next.relative_x();
            let delta_y = b_next.absolute_y() - b_iterator.absolute_y();
            let change = ((last_l_abs_y - b_iterator.absolute_y()) * delta_x) / delta_y;

            let new_x = -l_abs_x + r_abs_x + change;
            let new_next = OutlineNode::new(
                b_next.relative_x() - change,
                b_next.absolute_y(),
                b_next.next_cloned(),
            );
            let last_l = outline_tail_mut(&mut left_outline);
            last_l.set_next(Some(OutlineNode::new(new_x, last_l_abs_y, Some(new_next))));

            node_set_property(
                left_subtree,
                InternalProperties::LEFT_OUTLINE,
                Some(left_outline),
            );
            node_set_property(
                left_subtree,
                InternalProperties::OUTLINE_MAX_DEPTH,
                Some(b_max_depth),
            );
        }

        let b_max_depth = node_get_property(b, InternalProperties::OUTLINE_MAX_DEPTH).unwrap_or(0.0);
        let a_max_depth = node_get_property(a, InternalProperties::OUTLINE_MAX_DEPTH).unwrap_or(0.0);
        if b_max_depth < a_max_depth {
            let mut right_outline = node_get_property(b, InternalProperties::RIGHT_OUTLINE)
                .expect("right outline");
            let (r_abs_x, last_b_abs_y) = outline_abs_x_and_last_y(&right_outline, node_x(b));

            let mut a_iterator = {
                let a_right = node_get_property(a, InternalProperties::RIGHT_OUTLINE)
                    .expect("right outline");
                OutlineNode::new(a_right.relative_x(), MINIMAL_Y, a_right.next_cloned())
            };
            let mut a_abs_x = a_iterator.relative_x() + node_x(a);
            while a_iterator
                .next()
                .map(|node| node.absolute_y() <= last_b_abs_y)
                .unwrap_or(false)
            {
                let next = a_iterator.next_cloned().expect("next");
                a_abs_x += next.relative_x();
                a_iterator = next;
            }

            let a_next = a_iterator.next().expect("next");
            let delta_x = a_next.relative_x();
            let delta_y = a_next.absolute_y() - a_iterator.absolute_y();
            let change = ((last_b_abs_y - a_iterator.absolute_y()) * delta_x) / delta_y;

            let new_x = a_abs_x - r_abs_x + change;
            let new_next = OutlineNode::new(
                a_next.relative_x() - change,
                a_next.absolute_y(),
                a_next.next_cloned(),
            );
            let last_b = outline_tail_mut(&mut right_outline);
            last_b.set_next(Some(OutlineNode::new(new_x, last_b_abs_y, Some(new_next))));

            node_set_property(b, InternalProperties::RIGHT_OUTLINE, Some(right_outline));
            node_set_property(
                b,
                InternalProperties::OUTLINE_MAX_DEPTH,
                Some(a_max_depth),
            );
        }
    }
}

impl Default for RelativeXPlacer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<VertiFlexLayoutPhases, ElkNodeRef> for RelativeXPlacer {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("XPlacer", 1.0);

        self.spacing_node_node = node_get_property(graph, CoreOptions::SPACING_NODE_NODE).unwrap_or(0.0);
        self.consider_node_model_order =
            node_get_property(graph, VertiFlexOptions::CONSIDER_NODE_MODEL_ORDER).unwrap_or(true);

        let has_children = {
            let mut graph_mut = graph.borrow_mut();
            !graph_mut.children().is_empty()
        };
        if has_children {
            if let Some(parent) = VertiFlexUtil::find_root(graph) {
                match node_get_property(graph, VertiFlexOptions::LAYOUT_STRATEGY)
                    .unwrap_or_default()
                {
                    EdgeRoutingStrategy::Straight => self.recursive_straightline_placement(&parent),
                    EdgeRoutingStrategy::Bend => self.recursive_bentline_placement(&parent),
                }
            }
        }

        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &ElkNodeRef,
    ) -> Option<LayoutProcessorConfiguration<VertiFlexLayoutPhases, ElkNodeRef>> {
        None
    }
}

const MINIMAL_Y: f64 = -100.0;

fn node_get_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
) -> Option<T> {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    props.get_property(property)
}

fn node_set_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: Option<T>,
) {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    props.set_property(property, value);
}

fn node_x(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().x()
}

fn node_y(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().y()
}

fn node_width(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().width()
}

fn node_height(node: &ElkNodeRef) -> f64 {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().height()
}

fn node_set_x(node: &ElkNodeRef, value: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_x(value);
}

fn edge_target_node(edge: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeRef) -> Option<ElkNodeRef> {
    let edge_borrow = edge.borrow();
    let target = edge_borrow.targets_ro().get(0)?;
    drop(edge_borrow);
    ElkGraphUtil::connectable_shape_to_node(&target)
}

fn set_outline_third_next(outline: &mut OutlineNode, next: OutlineNode) {
    let Some(level1) = outline.next_mut() else { return; };
    let Some(level2) = level1.next_mut() else { return; };
    let Some(level3) = level2.next_mut() else { return; };
    level3.set_next(Some(next));
}

fn outline_abs_x_and_last_y(outline: &OutlineNode, base_x: f64) -> (f64, f64) {
    let mut abs_x = outline.relative_x() + base_x;
    let mut last_y = outline.absolute_y();
    let mut current = outline;
    while let Some(next) = current.next() {
        abs_x += next.relative_x();
        last_y = next.absolute_y();
        current = next;
    }
    (abs_x, last_y)
}

fn outline_tail_mut(outline: &mut OutlineNode) -> &mut OutlineNode {
    if outline.is_last() {
        return outline;
    }
    let next = outline.next_mut().expect("next");
    outline_tail_mut(next)
}
