use std::collections::VecDeque;
use std::rc::Rc;
use std::{cell::Cell, thread_local};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{
    GraphFeature, MapPropertyHolder,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkEdgeRef, ElkLabelRef, ElkNodeRef};

use crate::org::eclipse::elk::core::data::{DeprecatedLayoutOptionReplacer, LayoutAlgorithmData};
use crate::org::eclipse::elk::core::data::{LayoutAlgorithmResolver, LayoutMetaDataService};
use crate::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use crate::org::eclipse::elk::core::math::{ElkPadding, KVector};
use crate::org::eclipse::elk::core::options::{
    ContentAlignment, CoreOptions, HierarchyHandling, TopdownNodeTypes,
};
use crate::org::eclipse::elk::core::testing::TestController;
use crate::org::eclipse::elk::core::unsupported_configuration::UnsupportedConfigurationException;
use crate::org::eclipse::elk::core::util::{ElkUtil, IElkProgressMonitor};
use crate::org::eclipse::elk::core::validation::{GraphValidator, LayoutOptionValidator};

#[derive(Clone, Default)]
pub struct RecursiveGraphLayoutEngine;

impl RecursiveGraphLayoutEngine {
    pub fn new() -> Self {
        RecursiveGraphLayoutEngine
    }

    pub fn layout_with_test_controller(
        &mut self,
        layout_graph: &ElkNodeRef,
        test_controller: &mut TestController,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        let controller = Some(test_controller as *mut TestController);
        self.layout_internal(layout_graph, controller, progress_monitor);
    }

    fn layout_recursively(
        &mut self,
        layout_node: &ElkNodeRef,
        test_controller: Option<*mut TestController>,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) -> Vec<ElkEdgeRef> {
        let _trace_depth = RecursiveTraceDepthGuard::new();
        recursive_trace(layout_node, "enter");
        if progress_monitor.is_canceled() {
            recursive_trace(layout_node, "exit canceled");
            return Vec::new();
        }

        let no_layout = with_node_properties_mut(layout_node, |props| {
            props.get_property(CoreOptions::NO_LAYOUT).unwrap_or(false)
        });
        if no_layout {
            recursive_trace(layout_node, "exit no_layout=true");
            return Vec::new();
        }

        let children: Vec<ElkNodeRef> = {
            let mut node_mut = layout_node.borrow_mut();
            node_mut.children().iter().cloned().collect()
        };

        let inside_self_loops = self.gather_inside_self_loops(layout_node);
        let has_inside_self_loops = !inside_self_loops.is_empty();
        let has_children = !children.is_empty();
        recursive_trace(
            layout_node,
            &format!(
                "state children={} inside_self_loops={}",
                children.len(),
                inside_self_loops.len()
            ),
        );

        if !has_children && !has_inside_self_loops {
            recursive_trace(layout_node, "exit leaf without inside self loops");
            return Vec::new();
        }

        let Some(algorithm_data) = get_resolved_algorithm(layout_node) else {
            let message =
                "Resolved algorithm is not set; apply a LayoutAlgorithmResolver before computing layout.";
            panic!("{}", UnsupportedConfigurationException::new(message));
        };

        let supports_inside_self_loops =
            algorithm_data.supports_feature(GraphFeature::InsideSelfLoops);
        if !has_children && has_inside_self_loops && !supports_inside_self_loops {
            recursive_trace(layout_node, "exit inside self loops unsupported");
            return Vec::new();
        }

        self.evaluate_hierarchy_handling_inheritance(layout_node);
        let hierarchy_handling = with_node_properties_mut(layout_node, |props| {
            props
                .get_property(CoreOptions::HIERARCHY_HANDLING)
                .unwrap_or(HierarchyHandling::Inherit)
        });

        let include_children = hierarchy_handling == HierarchyHandling::IncludeChildren
            && (algorithm_data.supports_feature(GraphFeature::Compound)
                || algorithm_data.supports_feature(GraphFeature::Clusters));
        recursive_trace(
            layout_node,
            &format!(
                "config hierarchy_handling={hierarchy_handling:?} include_children={} algorithm={}",
                include_children,
                algorithm_data.id()
            ),
        );

        let topdown_layout = with_node_properties_mut(layout_node, |props| {
            props
                .get_property(CoreOptions::TOPDOWN_LAYOUT)
                .unwrap_or(false)
        });
        if include_children && topdown_layout {
            let message = "Topdown layout cannot be used together with hierarchy handling.";
            panic!("{}", UnsupportedConfigurationException::new(message));
        }

        let mut children_inside_self_loops: Vec<ElkEdgeRef> = Vec::new();
        let node_count: usize;

        if include_children {
            node_count = Self::count_nodes_with_hierarchy(layout_node);

            let mut queue: VecDeque<ElkNodeRef> = children.iter().cloned().collect();
            while let Some(node) = queue.pop_front() {
                self.evaluate_hierarchy_handling_inheritance(&node);
                let stop_hierarchy = with_node_properties_mut(&node, |props| {
                    props
                        .get_property(CoreOptions::HIERARCHY_HANDLING)
                        .unwrap_or(HierarchyHandling::SeparateChildren)
                }) == HierarchyHandling::SeparateChildren;

                let algorithm_switch = with_node_properties_mut(&node, |props| {
                    props.has_property(CoreOptions::ALGORITHM)
                }) && get_resolved_algorithm(&node)
                    .map(|child_algo| child_algo != algorithm_data)
                    .unwrap_or(false);

                if stop_hierarchy || algorithm_switch {
                    recursive_trace(
                        &node,
                        &format!(
                            "recurse include_children stop_hierarchy={} algorithm_switch={}",
                            stop_hierarchy, algorithm_switch
                        ),
                    );
                    let mut sub_monitor = progress_monitor.sub_task(1.0);
                    let child_loops =
                        self.layout_recursively(&node, test_controller, sub_monitor.as_mut());
                    children_inside_self_loops.extend(child_loops);

                    with_node_properties_mut(&node, |props| {
                        props.set_property(
                            CoreOptions::HIERARCHY_HANDLING,
                            Some(HierarchyHandling::SeparateChildren),
                        );
                    });

                    ElkUtil::apply_configured_node_scaling(&node);
                } else {
                    let grandchildren: Vec<ElkNodeRef> = {
                        let mut node_mut = node.borrow_mut();
                        node_mut.children().iter().cloned().collect()
                    };
                    recursive_trace(
                        &node,
                        &format!(
                            "include_children queue grandchildren={}",
                            grandchildren.len()
                        ),
                    );
                    for child in grandchildren {
                        queue.push_back(child);
                    }
                }
            }
        } else {
            node_count = children.len();
            if topdown_layout {
                self.apply_topdown_layout(
                    layout_node,
                    &children,
                    &algorithm_data,
                    node_count,
                    test_controller,
                    progress_monitor,
                );
            }

            for child in children {
                recursive_trace(&child, "recurse separate child");
                let mut sub_monitor = progress_monitor.sub_task(1.0);
                let child_loops =
                    self.layout_recursively(&child, test_controller, sub_monitor.as_mut());
                children_inside_self_loops.extend(child_loops);
                ElkUtil::apply_configured_node_scaling(&child);
            }
        }

        if progress_monitor.is_canceled() {
            recursive_trace(layout_node, "exit canceled after child processing");
            return Vec::new();
        }

        for edge in &children_inside_self_loops {
            with_edge_properties_mut(edge, |props| {
                props.set_property(CoreOptions::NO_LAYOUT, Some(true));
            });
        }

        if !topdown_layout {
            recursive_trace(
                layout_node,
                &format!(
                    "execute algorithm={} node_count={} include_children={}",
                    algorithm_data.id(),
                    node_count,
                    include_children
                ),
            );
            let mut sub_monitor = progress_monitor.sub_task(node_count as f32);
            self.execute_algorithm(
                layout_node,
                &algorithm_data,
                test_controller,
                sub_monitor.as_mut(),
            );
        }

        self.post_process_inside_self_loops(&children_inside_self_loops);

        let result = if has_inside_self_loops && supports_inside_self_loops {
            inside_self_loops
        } else {
            Vec::new()
        };
        recursive_trace(
            layout_node,
            &format!("exit returning_inside_self_loops={}", result.len()),
        );
        result
    }

    fn execute_algorithm(
        &mut self,
        layout_node: &ElkNodeRef,
        algorithm_data: &LayoutAlgorithmData,
        test_controller: Option<*mut TestController>,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        let Some(pool) = algorithm_data.provider_pool() else {
            let message = format!(
                "Layout algorithm '{}' is not available in this build.",
                algorithm_data.id()
            );
            panic!("{}", UnsupportedConfigurationException::new(message));
        };

        let mut provider = pool.fetch();
        let guard = if let Some(controller) = test_controller {
            let should_install = unsafe { (&*controller).targets(algorithm_data) };
            if should_install {
                let install_result = unsafe { (&mut *controller).install(provider.as_mut()) };
                if let Err(message) = install_result {
                    panic!("{message}");
                }
                Some(TestControllerGuard {
                    controller,
                    provider: provider.as_mut()
                        as *mut dyn crate::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider,
                })
            } else {
                None
            }
        } else {
            None
        };

        {
            let _guard = guard;
            provider.layout(layout_node, progress_monitor);
        }

        pool.release(provider);
    }

    fn apply_topdown_layout(
        &mut self,
        layout_node: &ElkNodeRef,
        children: &[ElkNodeRef],
        algorithm_data: &LayoutAlgorithmData,
        node_count: usize,
        test_controller: Option<*mut TestController>,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        let mut topdown_monitor = progress_monitor.sub_task(1.0);
        topdown_monitor.begin("Topdown Layout", 1.0);

        let node_type = with_node_properties_mut(layout_node, |props| {
            props.get_property(CoreOptions::TOPDOWN_NODE_TYPE)
        });
        let Some(node_type) = node_type else {
            let message = "Top-down layout node type has not been assigned.";
            panic!("{}", UnsupportedConfigurationException::new(message));
        };

        if node_type == TopdownNodeTypes::HierarchicalNode
            || node_type == TopdownNodeTypes::RootNode
        {
            for child in children {
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

                let mut handled = false;
                if let Some(local_algorithm_data) = get_resolved_algorithm(child) {
                    if let Some(pool) = local_algorithm_data.provider_pool() {
                        let provider = pool.fetch();
                        let required_size = provider
                            .as_topdown_layout_provider()
                            .map(|topdown_provider| {
                                let child_type = with_node_properties_mut(child, |props| {
                                    props.get_property(CoreOptions::TOPDOWN_NODE_TYPE)
                                });
                                if child_type == Some(TopdownNodeTypes::HierarchicalNode) {
                                    let message = "Topdown layout providers should only be used on parallel nodes.";
                                    panic!("{}", UnsupportedConfigurationException::new(message));
                                }
                                topdown_provider.get_predicted_graph_size(child)
                            });
                        pool.release(provider);

                        if let Some(required_size) = required_size {
                            set_node_dimensions_min(child, &required_size);
                            handled = true;
                        }
                    }
                }

                if handled {
                    continue;
                }

                let approximator = with_node_properties_mut(child, |props| {
                    props.get_property(CoreOptions::TOPDOWN_SIZE_APPROXIMATOR)
                });
                if let Some(approximator) = approximator {
                    let size = approximator.get_size(child);
                    set_node_dimensions_with_padding(child, &size, &padding);
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
                    set_node_dimensions_with_padding(child, &size, &padding);
                }
            }
        }

        let padding = with_node_properties_mut(layout_node, |props| {
            props
                .get_property(CoreOptions::PADDING)
                .unwrap_or_else(ElkPadding::new)
        });
        let (width, height) = node_dimensions(layout_node);
        let child_area_available_width = width - (padding.left + padding.right);
        let child_area_available_height = height - (padding.top + padding.bottom);
        topdown_monitor.log(&format!(
            "Available Child Area: ({child_area_available_width}|{child_area_available_height})"
        ));

        with_node_properties_mut(layout_node, |props| {
            props.set_property(
                CoreOptions::ASPECT_RATIO,
                Some(child_area_available_width / child_area_available_height),
            );
        });

        let fixed_graph_size = with_node_properties_mut(layout_node, |props| {
            props
                .get_property(CoreOptions::NODE_SIZE_FIXED_GRAPH_SIZE)
                .unwrap_or(false)
        });
        let original_node_size = node_dimensions(layout_node);

        let mut sub_monitor = progress_monitor.sub_task(node_count as f32);
        self.execute_algorithm(
            layout_node,
            algorithm_data,
            test_controller,
            sub_monitor.as_mut(),
        );

        if fixed_graph_size {
            let mut node_mut = layout_node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .set_dimensions(original_node_size.0, original_node_size.1);
        }

        if node_type == TopdownNodeTypes::RootNode {
            ElkUtil::compute_child_area_dimensions(layout_node);
            let child_area_width = with_node_properties_mut(layout_node, |props| {
                props
                    .get_property(CoreOptions::CHILD_AREA_WIDTH)
                    .unwrap_or(0.0)
            });
            let child_area_height = with_node_properties_mut(layout_node, |props| {
                props
                    .get_property(CoreOptions::CHILD_AREA_HEIGHT)
                    .unwrap_or(0.0)
            });
            let mut node_mut = layout_node.borrow_mut();
            let shape = node_mut.connectable().shape();
            shape.set_dimensions(
                padding.left + child_area_width + padding.right,
                padding.top + child_area_height + padding.bottom,
            );
        }

        let algorithm_id = with_node_properties_mut(layout_node, |props| {
            props.get_property(CoreOptions::ALGORITHM)
        })
        .unwrap_or_default();
        topdown_monitor.log(&format!(
            "Executed layout algorithm: {} on node",
            algorithm_id
        ));

        if node_type == TopdownNodeTypes::HierarchicalNode {
            if child_area_available_width < 0.0 || child_area_available_height < 0.0 {
                let message = "The size defined by the parent parallel node is too small for the \
                    space provided by the paddings of the child hierarchical node.";
                panic!("{}", UnsupportedConfigurationException::new(message));
            }

            let has_child_area = with_node_properties_mut(layout_node, |props| {
                props.has_property(CoreOptions::CHILD_AREA_WIDTH)
                    || props.has_property(CoreOptions::CHILD_AREA_HEIGHT)
            });
            if !has_child_area {
                ElkUtil::compute_child_area_dimensions(layout_node);
            }

            let child_area_desired_width = with_node_properties_mut(layout_node, |props| {
                props
                    .get_property(CoreOptions::CHILD_AREA_WIDTH)
                    .unwrap_or(0.0)
            });
            let child_area_desired_height = with_node_properties_mut(layout_node, |props| {
                props
                    .get_property(CoreOptions::CHILD_AREA_HEIGHT)
                    .unwrap_or(0.0)
            });
            topdown_monitor.log(&format!(
                "Desired Child Area: ({child_area_desired_width}|{child_area_desired_height})"
            ));

            let scale_factor_x = child_area_available_width / child_area_desired_width;
            let scale_factor_y = child_area_available_height / child_area_desired_height;
            let scale_cap = with_node_properties_mut(layout_node, |props| {
                props
                    .get_property(CoreOptions::TOPDOWN_SCALE_CAP)
                    .unwrap_or(1.0)
            });
            let scale_factor = scale_factor_x.min(scale_factor_y).min(scale_cap);
            with_node_properties_mut(layout_node, |props| {
                props.set_property(CoreOptions::TOPDOWN_SCALE_FACTOR, Some(scale_factor));
            });
            topdown_monitor.log(&format!(
                "Local Scale Factor (X|Y): ({scale_factor_x}|{scale_factor_y})"
            ));

            let content_alignment = with_node_properties_mut(layout_node, |props| {
                props
                    .get_property(CoreOptions::CONTENT_ALIGNMENT)
                    .unwrap_or_else(ContentAlignment::top_left)
            });

            let mut alignment_shift_x = 0.0;
            let mut alignment_shift_y = 0.0;

            if scale_factor < scale_factor_x {
                if content_alignment.contains(&ContentAlignment::HCenter) {
                    alignment_shift_x = (child_area_available_width / 2.0
                        - (child_area_desired_width * scale_factor) / 2.0)
                        / scale_factor;
                } else if content_alignment.contains(&ContentAlignment::HRight) {
                    alignment_shift_x = (child_area_available_width
                        - child_area_desired_width * scale_factor)
                        / scale_factor;
                }
            }

            if scale_factor < scale_factor_y {
                if content_alignment.contains(&ContentAlignment::VCenter) {
                    alignment_shift_y = (child_area_available_height / 2.0
                        - (child_area_desired_height * scale_factor) / 2.0)
                        / scale_factor;
                } else if content_alignment.contains(&ContentAlignment::VBottom) {
                    alignment_shift_y = (child_area_available_height
                        - child_area_desired_height * scale_factor)
                        / scale_factor;
                }
            }

            let x_shift = alignment_shift_x + (padding.left / scale_factor - padding.left);
            let y_shift = alignment_shift_y + (padding.top / scale_factor - padding.top);
            topdown_monitor.log(&format!("Shift: ({x_shift}|{y_shift})"));

            for child in children {
                shift_node_location(child, x_shift, y_shift);
            }

            let edges: Vec<ElkEdgeRef> = {
                let mut node_mut = layout_node.borrow_mut();
                node_mut.contained_edges().iter().cloned().collect()
            };
            for edge in edges {
                let sections: Vec<_> = {
                    let mut edge_mut = edge.borrow_mut();
                    edge_mut.sections().iter().cloned().collect()
                };
                for section in sections {
                    let mut section_mut = section.borrow_mut();
                    let start_x = section_mut.start_x();
                    let start_y = section_mut.start_y();
                    let end_x = section_mut.end_x();
                    let end_y = section_mut.end_y();
                    section_mut.set_start_x(start_x + x_shift);
                    section_mut.set_start_y(start_y + y_shift);
                    section_mut.set_end_x(end_x + x_shift);
                    section_mut.set_end_y(end_y + y_shift);

                    let points = section_mut.bend_points().to_vec();
                    for point in points {
                        let mut point_mut = point.borrow_mut();
                        let x = point_mut.x();
                        let y = point_mut.y();
                        point_mut.set_x(x + x_shift);
                        point_mut.set_y(y + y_shift);
                    }
                }

                let labels: Vec<ElkLabelRef> = {
                    let mut edge_mut = edge.borrow_mut();
                    edge_mut.element().labels().iter().cloned().collect()
                };
                for label in labels {
                    shift_label_location(&label, x_shift, y_shift);
                }

                with_edge_properties_mut(&edge, |props| {
                    if let Some(mut junction_points) =
                        props.get_property(CoreOptions::JUNCTION_POINTS)
                    {
                        junction_points.offset(x_shift, y_shift);
                        props.set_property(CoreOptions::JUNCTION_POINTS, Some(junction_points));
                    }
                });
            }
        }

        topdown_monitor.done();
    }

    fn count_nodes_recursively(layout_node: &ElkNodeRef, count_ancestors: bool) -> usize {
        let children: Vec<ElkNodeRef> = {
            let mut node_mut = layout_node.borrow_mut();
            node_mut.children().iter().cloned().collect()
        };

        let mut count = children.len();
        for child in &children {
            let has_children = {
                let mut child_mut = child.borrow_mut();
                !child_mut.children().is_empty()
            };
            if has_children {
                count += Self::count_nodes_recursively(child, false);
            }
        }

        if count_ancestors {
            let mut parent = layout_node.borrow().parent();
            while let Some(parent_node) = parent {
                let parent_children_count = {
                    let mut parent_mut = parent_node.borrow_mut();
                    parent_mut.children().len()
                };
                count += parent_children_count;
                parent = parent_node.borrow().parent();
            }
        }

        count
    }

    fn count_nodes_with_hierarchy(parent_node: &ElkNodeRef) -> usize {
        let children: Vec<ElkNodeRef> = {
            let mut node_mut = parent_node.borrow_mut();
            node_mut.children().iter().cloned().collect()
        };

        let mut count = children.len();
        for child in children {
            let handling = with_node_properties_mut(&child, |props| {
                props
                    .get_property(CoreOptions::HIERARCHY_HANDLING)
                    .unwrap_or(HierarchyHandling::SeparateChildren)
            });
            if handling == HierarchyHandling::SeparateChildren {
                continue;
            }

            let parent_algo = get_resolved_algorithm(parent_node);
            let child_algo = get_resolved_algorithm(&child);
            let same_algo = match (parent_algo, child_algo) {
                (Some(parent_algo), Some(child_algo)) => parent_algo == child_algo,
                _ => false,
            };

            let has_children = {
                let mut child_mut = child.borrow_mut();
                !child_mut.children().is_empty()
            };
            if same_algo && has_children {
                count += Self::count_nodes_with_hierarchy(&child);
            }
        }

        count
    }

    fn evaluate_hierarchy_handling_inheritance(&self, layout_node: &ElkNodeRef) {
        let handling = with_node_properties_mut(layout_node, |props| {
            props
                .get_property(CoreOptions::HIERARCHY_HANDLING)
                .unwrap_or(HierarchyHandling::Inherit)
        });

        if handling == HierarchyHandling::Inherit {
            let parent = layout_node.borrow().parent();
            if let Some(parent) = parent {
                let parent_handling = with_node_properties_mut(&parent, |props| {
                    props
                        .get_property(CoreOptions::HIERARCHY_HANDLING)
                        .unwrap_or(HierarchyHandling::SeparateChildren)
                });
                with_node_properties_mut(layout_node, |props| {
                    props.set_property(CoreOptions::HIERARCHY_HANDLING, Some(parent_handling));
                });
            } else {
                with_node_properties_mut(layout_node, |props| {
                    props.set_property(
                        CoreOptions::HIERARCHY_HANDLING,
                        Some(HierarchyHandling::SeparateChildren),
                    );
                });
            }
        }
    }

    fn gather_inside_self_loops(&self, node: &ElkNodeRef) -> Vec<ElkEdgeRef> {
        let active = with_node_properties_mut(node, |props| {
            props
                .get_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE)
                .unwrap_or(false)
        });
        if !active {
            return Vec::new();
        }

        let mut inside_self_loops = Vec::new();
        for edge in ElkGraphUtil::all_outgoing_edges(node) {
            let is_self_loop = edge.borrow().is_selfloop();
            if !is_self_loop {
                continue;
            }
            let inside = with_edge_properties_mut(&edge, |props| {
                props
                    .get_property(CoreOptions::INSIDE_SELF_LOOPS_YO)
                    .unwrap_or(false)
            });
            if inside {
                inside_self_loops.push(edge);
            }
        }

        inside_self_loops
    }

    fn post_process_inside_self_loops(&self, inside_self_loops: &[ElkEdgeRef]) {
        for edge in inside_self_loops {
            let node = {
                let edge_borrow = edge.borrow();
                let maybe_node = edge_borrow
                    .sources_ro()
                    .iter()
                    .next()
                    .and_then(ElkGraphUtil::connectable_shape_to_node);
                maybe_node
            };

            let Some(node) = node else {
                continue;
            };

            let (x_offset, y_offset) = {
                let mut node_mut = node.borrow_mut();
                let shape = node_mut.connectable().shape();
                (shape.x(), shape.y())
            };

            let sections: Vec<_> = {
                let mut edge_mut = edge.borrow_mut();
                edge_mut.sections().iter().cloned().collect()
            };
            for section in sections {
                let mut section_mut = section.borrow_mut();
                let start_x = section_mut.start_x();
                let start_y = section_mut.start_y();
                let end_x = section_mut.end_x();
                let end_y = section_mut.end_y();
                section_mut.set_start_x(start_x + x_offset);
                section_mut.set_start_y(start_y + y_offset);
                section_mut.set_end_x(end_x + x_offset);
                section_mut.set_end_y(end_y + y_offset);

                let points = section_mut.bend_points().to_vec();
                for point in points {
                    let mut point_mut = point.borrow_mut();
                    let x = point_mut.x();
                    let y = point_mut.y();
                    point_mut.set_x(x + x_offset);
                    point_mut.set_y(y + y_offset);
                }
            }

            with_edge_properties_mut(edge, |props| {
                if let Some(mut points) = props.get_property(CoreOptions::JUNCTION_POINTS) {
                    points.offset(x_offset, y_offset);
                    props.set_property(CoreOptions::JUNCTION_POINTS, Some(points));
                }
            });
        }
    }

    fn layout_internal(
        &mut self,
        layout_graph: &ElkNodeRef,
        test_controller: Option<*mut TestController>,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        LayoutMetaDataService::get_instance();
        let node_count = Self::count_nodes_recursively(layout_graph, true);
        progress_monitor.begin("Recursive Graph Layout", node_count as f32);

        let mut deprecated_replacer = DeprecatedLayoutOptionReplacer::new();
        ElkUtil::apply_visitors(layout_graph, &mut [&mut deprecated_replacer]);

        let validate_graph = with_node_properties_mut(layout_graph, |props| {
            props
                .get_property(CoreOptions::VALIDATE_GRAPH)
                .unwrap_or(false)
        });
        let validate_options = with_node_properties_mut(layout_graph, |props| {
            props
                .get_property(CoreOptions::VALIDATE_OPTIONS)
                .unwrap_or(false)
        });

        let mut resolver = LayoutAlgorithmResolver::new();
        {
            let mut option_validator = LayoutOptionValidator::new();
            let mut graph_validator = GraphValidator::new();
            let mut visitors: Vec<
                &mut dyn crate::org::eclipse::elk::core::util::IGraphElementVisitor,
            > = Vec::new();
            visitors.push(&mut resolver);
            if validate_options {
                visitors.push(&mut option_validator);
            }
            if validate_graph {
                visitors.push(&mut graph_validator);
            }

            if validate_graph || validate_options {
                if let Err(error) =
                    ElkUtil::apply_visitors_with_validation(layout_graph, &mut visitors)
                {
                    panic!("{}", error);
                }
            } else {
                ElkUtil::apply_visitors(layout_graph, &mut visitors);
            }
        }

        if let Some(error) = resolver.errors().first() {
            panic!("{}", error);
        }

        self.layout_recursively(layout_graph, test_controller, progress_monitor);

        progress_monitor.done();
    }
}

impl IGraphLayoutEngine for RecursiveGraphLayoutEngine {
    fn layout(
        &mut self,
        layout_graph: &ElkNodeRef,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        self.layout_internal(layout_graph, None, progress_monitor);
    }
}

struct TestControllerGuard {
    controller: *mut TestController,
    provider:
        *mut dyn crate::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider,
}

impl Drop for TestControllerGuard {
    fn drop(&mut self) {
        unsafe {
            (&mut *self.controller).uninstall_from(&mut *self.provider);
        }
    }
}

fn get_resolved_algorithm(node: &ElkNodeRef) -> Option<LayoutAlgorithmData> {
    with_node_properties_mut(node, |props| {
        props.get_property(CoreOptions::RESOLVED_ALGORITHM)
    })
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

fn with_edge_properties_mut<R>(
    edge: &ElkEdgeRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut edge_mut = edge.borrow_mut();
    let props = edge_mut.element().properties_mut();
    f(props)
}

thread_local! {
    static TRACE_RECURSIVE_DEPTH: Cell<usize> = const { Cell::new(0) };
}

struct RecursiveTraceDepthGuard {
    enabled: bool,
}

impl RecursiveTraceDepthGuard {
    fn new() -> Self {
        let enabled = recursive_trace_enabled();
        if enabled {
            TRACE_RECURSIVE_DEPTH.with(|depth| depth.set(depth.get() + 1));
        }
        Self { enabled }
    }
}

impl Drop for RecursiveTraceDepthGuard {
    fn drop(&mut self) {
        if self.enabled {
            TRACE_RECURSIVE_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
        }
    }
}

static TRACE_RECURSIVE_LAYOUT: std::sync::LazyLock<bool> =
    std::sync::LazyLock::new(|| std::env::var_os("ELK_TRACE_RECURSIVE_LAYOUT").is_some());

fn recursive_trace_enabled() -> bool {
    *TRACE_RECURSIVE_LAYOUT
}

fn recursive_trace(node: &ElkNodeRef, message: &str) {
    if !recursive_trace_enabled() {
        return;
    }
    TRACE_RECURSIVE_DEPTH.with(|depth| {
        let indent = "  ".repeat(depth.get());
        eprintln!(
            "recursive-layout: {}node_ptr={:p} {}",
            indent,
            Rc::as_ptr(node),
            message
        );
    });
}

fn node_dimensions(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
}

fn set_node_dimensions_min(node: &ElkNodeRef, size: &KVector) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    shape.set_dimensions(shape.width().max(size.x), shape.height().max(size.y));
}

fn set_node_dimensions_with_padding(node: &ElkNodeRef, size: &KVector, padding: &ElkPadding) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    let width = shape.width().max(size.x + padding.left + padding.right);
    let height = shape.height().max(size.y + padding.top + padding.bottom);
    shape.set_dimensions(width, height);
}

fn shift_node_location(node: &ElkNodeRef, dx: f64, dy: f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    let x = shape.x();
    let y = shape.y();
    shape.set_location(x + dx, y + dy);
}

fn shift_label_location(label: &ElkLabelRef, dx: f64, dy: f64) {
    let mut label_mut = label.borrow_mut();
    let shape = label_mut.shape();
    let x = shape.x();
    let y = shape.y();
    shape.set_location(x + dx, y + dy);
}
