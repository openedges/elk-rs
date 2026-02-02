use std::rc::Rc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdge, ElkEdgeRef, ElkGraphElementRef, ElkGraphFactory, ElkNode,
    ElkNodeRef,
};

use crate::org::eclipse::elk::core::data::LayoutAlgorithmResolver;
use crate::org::eclipse::elk::core::math::{ElkPadding, KVector};
use crate::org::eclipse::elk::core::options::{
    CoreOptions, FixedLayouterOptions, TopdownSizeApproximatorUtil,
};
use crate::org::eclipse::elk::core::unsupported_configuration::UnsupportedConfigurationException;
use crate::org::eclipse::elk::core::util::{ElkUtil, IGraphElementVisitor, NullElkProgressMonitor};

pub trait ITopdownSizeApproximator: Send + Sync {
    fn get_size(&self, node: &ElkNodeRef) -> KVector;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TopdownSizeApproximator {
    CountChildren,
    LookaheadLayout,
    FixedIntegerRatioBoxes,
    LayoutNextLevel,
}

impl ITopdownSizeApproximator for TopdownSizeApproximator {
    fn get_size(&self, node: &ElkNodeRef) -> KVector {
        match self {
            TopdownSizeApproximator::CountChildren => count_children_size(node),
            TopdownSizeApproximator::LookaheadLayout => lookahead_layout_size(node),
            TopdownSizeApproximator::FixedIntegerRatioBoxes => fixed_integer_ratio_boxes_size(node),
            TopdownSizeApproximator::LayoutNextLevel => layout_next_level_size(node),
        }
    }
}

fn count_children_size(node: &ElkNodeRef) -> KVector {
    let child_count = {
        let mut node_mut = node.borrow_mut();
        node_mut.children().len()
    } as f64;
    let width = with_node_properties_mut(node, |props| {
        props
            .get_property(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH)
            .unwrap_or(150.0)
    });
    let aspect_ratio = with_node_properties_mut(node, |props| {
        props
            .get_property(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO)
            .unwrap_or(1.414)
    });
    let size = width * child_count.sqrt();
    KVector::with_values(size, size / aspect_ratio)
}

fn lookahead_layout_size(original_graph: &ElkNodeRef) -> KVector {
    let algorithm_data = with_node_properties_mut(original_graph, |props| {
        props.get_property(CoreOptions::RESOLVED_ALGORITHM)
    });
    let Some(algorithm_data) = algorithm_data else {
        let message =
            "Resolved algorithm is not set; apply a LayoutAlgorithmResolver before computing layout.";
        panic!("{}", UnsupportedConfigurationException::new(message));
    };

    let graph_factory = ElkGraphFactory::instance();
    let node = graph_factory.create_elk_node();
    copy_node_properties(original_graph, &node);

    let mut old_to_new: Vec<(ElkNodeRef, ElkNodeRef)> = Vec::new();
    let children: Vec<ElkNodeRef> = {
        let mut original_mut = original_graph.borrow_mut();
        original_mut.children().iter().cloned().collect()
    };

    for child in &children {
        let new_child = graph_factory.create_elk_node();
        ElkNode::set_parent(&new_child, Some(node.clone()));
        copy_node_properties(child, &new_child);

        let size = count_children_size(child);
        let (width, height) = node_dimensions(child);
        {
            let mut new_child_mut = new_child.borrow_mut();
            let shape = new_child_mut.connectable().shape();
            shape.set_dimensions(width.max(size.x), height.max(size.y));
        }
        old_to_new.push((child.clone(), new_child));
    }

    for child in &children {
        let edges: Vec<ElkEdgeRef> = {
            let mut child_mut = child.borrow_mut();
            child_mut.connectable().outgoing_edges().iter().collect()
        };
        for edge in edges {
            let target = {
                let edge_borrow = edge.borrow();
                edge_borrow
                    .targets_ro()
                    .get(0)
                    .and_then(|shape| ElkGraphUtil::connectable_shape_to_node(&shape))
            };

            let Some(target) = target else {
                continue;
            };

            let Some(new_src) = find_new_node(&old_to_new, child) else {
                continue;
            };
            let Some(new_tar) = find_new_node(&old_to_new, &target) else {
                continue;
            };

            let new_edge = graph_factory.create_elk_edge();
            ElkEdge::add_source(&new_edge, ElkConnectableShapeRef::Node(new_src.clone()));
            ElkEdge::add_target(&new_edge, ElkConnectableShapeRef::Node(new_tar.clone()));
            ElkEdge::set_containing_node(&new_edge, Some(node.clone()));
            copy_edge_properties(&edge, &new_edge);
        }
    }

    let Some(pool) = algorithm_data.provider_pool() else {
        let message = format!(
            "Layout algorithm '{}' is not available in this build.",
            algorithm_data.id()
        );
        panic!("{}", UnsupportedConfigurationException::new(message));
    };
    let mut provider = pool.fetch();
    let mut progress_monitor = NullElkProgressMonitor;
    provider.layout(&node, &mut progress_monitor);
    pool.release(provider);

    let has_child_area = with_node_properties_mut(&node, |props| {
        props.has_property(CoreOptions::CHILD_AREA_WIDTH)
            || props.has_property(CoreOptions::CHILD_AREA_HEIGHT)
    });
    if !has_child_area {
        ElkUtil::compute_child_area_dimensions(&node);
    }

    let child_area_width = with_node_properties_mut(&node, |props| {
        props
            .get_property(CoreOptions::CHILD_AREA_WIDTH)
            .unwrap_or(0.0)
    });
    let child_area_height = with_node_properties_mut(&node, |props| {
        props
            .get_property(CoreOptions::CHILD_AREA_HEIGHT)
            .unwrap_or(0.0)
    });
    let child_area_aspect_ratio = child_area_width / child_area_height;

    let base_width = with_node_properties_mut(&node, |props| {
        props
            .get_property(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH)
            .unwrap_or(150.0)
    });
    let base_size = base_width * (children.len() as f64).sqrt();

    let padding = with_node_properties_mut(&node, |props| {
        props
            .get_property(CoreOptions::PADDING)
            .unwrap_or_else(ElkPadding::new)
    });
    let min_width = padding.left + padding.right + 1.0;
    let min_height = padding.top + padding.bottom + 1.0;

    KVector::with_values(
        min_width.max(base_size),
        min_height.max(base_size / child_area_aspect_ratio),
    )
}

fn fixed_integer_ratio_boxes_size(original_graph: &ElkNodeRef) -> KVector {
    let base_width = with_node_properties_mut(original_graph, |props| {
        props
            .get_property(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH)
            .unwrap_or(150.0)
    });
    let aspect_ratio = with_node_properties_mut(original_graph, |props| {
        props
            .get_property(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO)
            .unwrap_or(1.414)
    });
    let base_height = base_width / aspect_ratio;

    let multiplier = TopdownSizeApproximatorUtil::get_size_category_multiplier(original_graph);

    let padding = with_node_properties_mut(original_graph, |props| {
        props
            .get_property(CoreOptions::PADDING)
            .unwrap_or_else(ElkPadding::new)
    });
    let node_node_spacing = original_graph
        .borrow()
        .parent()
        .map(|parent| {
            with_node_properties_mut(&parent, |props| {
                props
                    .get_property(CoreOptions::SPACING_NODE_NODE)
                    .unwrap_or(20.0)
            })
        })
        .unwrap_or_else(|| {
            CoreOptions::SPACING_NODE_NODE
                .get_default()
                .unwrap_or(20.0)
        });

    let mut result = KVector::with_values(base_width, base_height);
    result.scale(multiplier);
    result.add_values(
        -(padding.left + padding.right) - node_node_spacing,
        -(padding.top + padding.bottom) - node_node_spacing,
    );
    result
}

fn layout_next_level_size(original_graph: &ElkNodeRef) -> KVector {
    let children: Vec<ElkNodeRef> = {
        let mut node_mut = original_graph.borrow_mut();
        node_mut.children().iter().cloned().collect()
    };

    for child in &children {
        let has_children = {
            let mut child_mut = child.borrow_mut();
            !child_mut.children().is_empty()
        };
        if !has_children {
            continue;
        }

        let padding = with_node_properties_mut(child, |props| {
            props
                .get_property(CoreOptions::PADDING)
                .unwrap_or_else(ElkPadding::new)
        });

        let approximator = with_node_properties_mut(child, |props| {
            props.get_property(CoreOptions::TOPDOWN_SIZE_APPROXIMATOR)
        });

        if let Some(approximator) = approximator {
            let size = approximator.get_size(child);
            let (width, height) = node_dimensions(child);
            let mut child_mut = child.borrow_mut();
            let shape = child_mut.connectable().shape();
            shape.set_dimensions(
                width.max(size.x + padding.left + padding.right),
                height.max(size.y + padding.top + padding.bottom),
            );
        } else {
            let width = with_node_properties_mut(child, |props| {
                props
                    .get_property(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_WIDTH)
                    .unwrap_or(150.0)
            });
            let aspect_ratio = with_node_properties_mut(child, |props| {
                props
                    .get_property(CoreOptions::TOPDOWN_HIERARCHICAL_NODE_ASPECT_RATIO)
                    .unwrap_or(1.414)
            });
            let size = KVector::with_values(width, width / aspect_ratio);
            let (current_width, current_height) = node_dimensions(child);
            let mut child_mut = child.borrow_mut();
            let shape = child_mut.connectable().shape();
            shape.set_dimensions(
                current_width.max(size.x + padding.left + padding.right),
                current_height.max(size.y + padding.top + padding.bottom),
            );
        }
    }

    let algorithm_data = with_node_properties_mut(original_graph, |props| {
        props.get_property(CoreOptions::RESOLVED_ALGORITHM)
    });
    let Some(algorithm_data) = algorithm_data else {
        let message =
            "Resolved algorithm is not set; apply a LayoutAlgorithmResolver before computing layout.";
        panic!("{}", UnsupportedConfigurationException::new(message));
    };

    let Some(pool) = algorithm_data.provider_pool() else {
        let message = format!(
            "Layout algorithm '{}' is not available in this build.",
            algorithm_data.id()
        );
        panic!("{}", UnsupportedConfigurationException::new(message));
    };

    let mut provider = pool.fetch();
    let mut progress_monitor = NullElkProgressMonitor;
    provider.layout(original_graph, &mut progress_monitor);
    pool.release(provider);

    with_node_properties_mut(original_graph, |props| {
        props.set_property(
            CoreOptions::ALGORITHM,
            Some(FixedLayouterOptions::ALGORITHM_ID.to_string()),
        );
    });
    let mut resolver = LayoutAlgorithmResolver::new();
    resolver.visit(&ElkGraphElementRef::Node(original_graph.clone()));

    ElkUtil::compute_child_area_dimensions(original_graph);
    let child_area_width = with_node_properties_mut(original_graph, |props| {
        props
            .get_property(CoreOptions::CHILD_AREA_WIDTH)
            .unwrap_or(0.0)
    });
    let child_area_height = with_node_properties_mut(original_graph, |props| {
        props
            .get_property(CoreOptions::CHILD_AREA_HEIGHT)
            .unwrap_or(0.0)
    });
    KVector::with_values(child_area_width, child_area_height)
}

fn node_dimensions(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
}

fn copy_node_properties(source: &ElkNodeRef, target: &ElkNodeRef) {
    let source_props = {
        let mut source_mut = source.borrow_mut();
        source_mut.connectable().shape().graph_element().properties_mut().clone()
    };
    let mut target_mut = target.borrow_mut();
    *target_mut.connectable().shape().graph_element().properties_mut() = source_props;
}

fn copy_edge_properties(source: &ElkEdgeRef, target: &ElkEdgeRef) {
    let source_props = {
        let mut source_mut = source.borrow_mut();
        source_mut.element().properties_mut().clone()
    };
    let mut target_mut = target.borrow_mut();
    *target_mut.element().properties_mut() = source_props;
}

fn find_new_node(
    mapping: &[(ElkNodeRef, ElkNodeRef)],
    node: &ElkNodeRef,
) -> Option<ElkNodeRef> {
    mapping
        .iter()
        .find(|(old, _)| Rc::ptr_eq(old, node))
        .map(|(_, new)| new.clone())
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
