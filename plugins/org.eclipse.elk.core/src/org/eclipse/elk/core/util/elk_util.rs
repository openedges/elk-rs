use std::collections::HashMap;
use std::env;
use std::path::MAIN_SEPARATOR;
use std::rc::Rc;
use std::sync::LazyLock;

static TRACE_SIZING: LazyLock<bool> =
    LazyLock::new(|| std::env::var("ELK_TRACE_SIZING").is_ok());

use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkBendPointRef, ElkConnectableShapeRef, ElkEdgeRef, ElkEdgeSectionRef, ElkGraphElementRef,
    ElkGraphFactory, ElkLabelRef, ElkNodeRef, ElkPortRef,
};

use crate::org::eclipse::elk::core::data::LayoutMetaDataService;
use crate::org::eclipse::elk::core::math::{ElkMargin, ElkRectangle, KVector, KVectorChain};
use crate::org::eclipse::elk::core::options::{
    ContentAlignment, CoreOptions, Direction, EdgeLabelPlacement, NodeLabelPlacement,
    PortConstraints, PortSide, SizeConstraint, SizeOptions,
};
use crate::org::eclipse::elk::core::util::{EnumSet, IGraphElementVisitor};
use crate::org::eclipse::elk::core::validation::{GraphIssue, GraphValidationException, Severity};

pub struct ElkUtil;

pub trait TranslateArgs {
    fn translate(self);
}

pub trait ConfigureWithDefaultValuesArgs {
    fn configure(self);
}

impl ElkUtil {
    pub const DEFAULT_MIN_WIDTH: f64 = 20.0;
    pub const DEFAULT_MIN_HEIGHT: f64 = 20.0;

    pub fn translate<A: TranslateArgs>(args: A) {
        args.translate();
    }

    pub fn calc_port_side(port: &ElkPortRef, direction: Direction) -> PortSide {
        let parent = port
            .borrow()
            .parent()
            .expect("port must have a parent node to calculate the port side");

        let (node_width, node_height) = {
            let mut parent_mut = parent.borrow_mut();
            let shape = parent_mut.connectable().shape();
            (shape.width(), shape.height())
        };
        if node_width <= 0.0 && node_height <= 0.0 {
            return PortSide::Undefined;
        }

        let (xpos, ypos, port_width, port_height) = {
            let mut port_mut = port.borrow_mut();
            let shape = port_mut.connectable().shape();
            (shape.x(), shape.y(), shape.width(), shape.height())
        };

        match direction {
            Direction::Left | Direction::Right => {
                if xpos < 0.0 {
                    return PortSide::West;
                } else if xpos + port_width > node_width {
                    return PortSide::East;
                }
            }
            Direction::Up | Direction::Down => {
                if ypos < 0.0 {
                    return PortSide::North;
                } else if ypos + port_height > node_height {
                    return PortSide::South;
                }
            }
            Direction::Undefined => {}
        }

        let width_percent = (xpos + port_width / 2.0) / node_width;
        let height_percent = (ypos + port_height / 2.0) / node_height;
        if width_percent + height_percent <= 1.0 && width_percent - height_percent <= 0.0 {
            PortSide::West
        } else if width_percent + height_percent >= 1.0 && width_percent - height_percent >= 0.0 {
            PortSide::East
        } else if height_percent < 0.5 {
            PortSide::North
        } else {
            PortSide::South
        }
    }

    pub fn calc_port_offset(port: &ElkPortRef, side: PortSide) -> f64 {
        let parent = port
            .borrow()
            .parent()
            .expect("port must have a parent node to calculate the port side");

        let (node_width, node_height) = {
            let mut parent_mut = parent.borrow_mut();
            let shape = parent_mut.connectable().shape();
            (shape.width(), shape.height())
        };

        let (xpos, ypos, port_width, port_height) = {
            let mut port_mut = port.borrow_mut();
            let shape = port_mut.connectable().shape();
            (shape.x(), shape.y(), shape.width(), shape.height())
        };

        match side {
            PortSide::North => -(ypos + port_height),
            PortSide::East => xpos - node_width,
            PortSide::South => ypos - node_height,
            PortSide::West => -(xpos + port_width),
            PortSide::Undefined => 0.0,
        }
    }

    pub fn resize_node(node: &ElkNodeRef) -> Option<KVector> {
        LayoutMetaDataService::get_instance();
        let size_constraint = {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
        }
        .unwrap_or_else(SizeConstraint::fixed);

        if size_constraint.is_empty() {
            return None;
        }

        let mut new_width = 0.0;
        let mut new_height = 0.0;

        if size_constraint.contains(&SizeConstraint::Ports) {
            let port_constraints = {
                let mut node_mut = node.borrow_mut();
                node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(CoreOptions::PORT_CONSTRAINTS)
            }
            .unwrap_or(PortConstraints::Undefined);

            let direction = {
                let parent = node.borrow().parent();
                let target = parent.as_ref().unwrap_or(node);
                let mut target_mut = target.borrow_mut();
                target_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(CoreOptions::DIRECTION)
            }
            .unwrap_or(Direction::Undefined);

            let mut min_north: f64 = 2.0;
            let mut min_east: f64 = 2.0;
            let mut min_south: f64 = 2.0;
            let mut min_west: f64 = 2.0;

            let ports: Vec<ElkPortRef> = {
                let mut node_mut = node.borrow_mut();
                node_mut.ports().iter().cloned().collect()
            };
            for port in ports {
                let port_side = {
                    let mut port_mut = port.borrow_mut();
                    port_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .get_property(CoreOptions::PORT_SIDE)
                }
                .unwrap_or(PortSide::Undefined);

                let port_side = if port_side == PortSide::Undefined {
                    let calculated = Self::calc_port_side(&port, direction);
                    let mut port_mut = port.borrow_mut();
                    port_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .set_property(CoreOptions::PORT_SIDE, Some(calculated));
                    calculated
                } else {
                    port_side
                };

                let (xpos, ypos, port_width, port_height) = {
                    let mut port_mut = port.borrow_mut();
                    let shape = port_mut.connectable().shape();
                    (shape.x(), shape.y(), shape.width(), shape.height())
                };

                if port_constraints == PortConstraints::FixedPos {
                    match port_side {
                        PortSide::North => min_north = min_north.max(xpos + port_width),
                        PortSide::East => min_east = min_east.max(ypos + port_height),
                        PortSide::South => min_south = min_south.max(xpos + port_width),
                        PortSide::West => min_west = min_west.max(ypos + port_height),
                        PortSide::Undefined => {}
                    }
                } else {
                    match port_side {
                        PortSide::North => min_north += port_width + 2.0,
                        PortSide::East => min_east += port_height + 2.0,
                        PortSide::South => min_south += port_width + 2.0,
                        PortSide::West => min_west += port_height + 2.0,
                        PortSide::Undefined => {}
                    }
                }
            }

            new_width = min_north.max(min_south);
            new_height = min_east.max(min_west);
        }

        Some(Self::resize_node_with(
            node, new_width, new_height, true, true,
        ))
    }

    pub fn resize_node_with(
        node: &ElkNodeRef,
        new_width: f64,
        new_height: f64,
        move_ports: bool,
        move_labels: bool,
    ) -> KVector {
        LayoutMetaDataService::get_instance();
        let old_size = {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            KVector::with_values(shape.width(), shape.height())
        };

        let mut new_size = Self::effective_min_size_constraint_for(node);
        if new_width > new_size.x {
            new_size.x = new_width;
        }
        if new_height > new_size.y {
            new_size.y = new_height;
        }

        let width_ratio = new_size.x / old_size.x;
        let height_ratio = new_size.y / old_size.y;
        let width_diff = new_size.x - old_size.x;
        let height_diff = new_size.y - old_size.y;

        if move_ports {
            let direction = {
                let parent = node.borrow().parent();
                let target = parent.as_ref().unwrap_or(node);
                let mut target_mut = target.borrow_mut();
                target_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(CoreOptions::DIRECTION)
            }
            .unwrap_or(Direction::Undefined);

            let fixed_ports = {
                let mut node_mut = node.borrow_mut();
                node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(CoreOptions::PORT_CONSTRAINTS)
            }
            .unwrap_or(PortConstraints::Undefined)
                == PortConstraints::FixedPos;

            let ports: Vec<ElkPortRef> = {
                let mut node_mut = node.borrow_mut();
                node_mut.ports().iter().cloned().collect()
            };
            for port in ports {
                let port_side = {
                    let mut port_mut = port.borrow_mut();
                    port_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .get_property(CoreOptions::PORT_SIDE)
                }
                .unwrap_or(PortSide::Undefined);

                let port_side = if port_side == PortSide::Undefined {
                    let calculated = Self::calc_port_side(&port, direction);
                    let mut port_mut = port.borrow_mut();
                    port_mut
                        .connectable()
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .set_property(CoreOptions::PORT_SIDE, Some(calculated));
                    calculated
                } else {
                    port_side
                };

                let mut port_mut = port.borrow_mut();
                let shape = port_mut.connectable().shape();
                let x = shape.x();
                let y = shape.y();

                match port_side {
                    PortSide::North => {
                        if !fixed_ports {
                            shape.set_x(x * width_ratio);
                        }
                    }
                    PortSide::East => {
                        shape.set_x(x + width_diff);
                        if !fixed_ports {
                            shape.set_y(y * height_ratio);
                        }
                    }
                    PortSide::South => {
                        if !fixed_ports {
                            shape.set_x(x * width_ratio);
                        }
                        shape.set_y(y + height_diff);
                    }
                    PortSide::West => {
                        if !fixed_ports {
                            shape.set_y(y * height_ratio);
                        }
                    }
                    PortSide::Undefined => {}
                }
            }
        }

        if *TRACE_SIZING {
            eprintln!("TRACE resize_node_with: old=({:.1},{:.1}) input=({:.1},{:.1}) new=({:.1},{:.1})",
                old_size.x, old_size.y, new_width, new_height, new_size.x, new_size.y);
        }
        {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .set_dimensions(new_size.x, new_size.y);
        }

        if move_labels {
            let labels: Vec<ElkLabelRef> = {
                let mut node_mut = node.borrow_mut();
                node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect()
            };
            for label in labels {
                let mut label_mut = label.borrow_mut();
                let shape = label_mut.shape();
                let midx = shape.x() + shape.width() / 2.0;
                let midy = shape.y() + shape.height() / 2.0;
                let width_percent = midx / old_size.x;
                let height_percent = midy / old_size.y;

                if width_percent + height_percent >= 1.0 {
                    if width_percent - height_percent > 0.0 && midy >= 0.0 {
                        shape.set_x(shape.x() + width_diff);
                        shape.set_y(shape.y() + height_diff * height_percent);
                    } else if width_percent - height_percent < 0.0 && midx >= 0.0 {
                        shape.set_x(shape.x() + width_diff * width_percent);
                        shape.set_y(shape.y() + height_diff);
                    }
                }
            }
        }

        {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .set_property(
                    CoreOptions::NODE_SIZE_CONSTRAINTS,
                    Some(SizeConstraint::fixed()),
                );
        }

        KVector::with_values(width_ratio, height_ratio)
    }

    pub fn effective_min_size_constraint_for(node: &ElkNodeRef) -> KVector {
        LayoutMetaDataService::get_instance();
        let size_constraint = {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
        }
        .unwrap_or_else(SizeConstraint::fixed);

        if size_constraint.contains(&SizeConstraint::MinimumSize) {
            let size_options = {
                let mut node_mut = node.borrow_mut();
                node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(CoreOptions::NODE_SIZE_OPTIONS)
            }
            .unwrap_or_else(EnumSet::none_of);

            let mut min_size = {
                let mut node_mut = node.borrow_mut();
                node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .properties_mut()
                    .get_property(CoreOptions::NODE_SIZE_MINIMUM)
            }
            .unwrap_or_else(KVector::new);

            if size_options.contains(&SizeOptions::DefaultMinimumSize) {
                if min_size.x <= 0.0 {
                    min_size.x = Self::DEFAULT_MIN_WIDTH;
                }
                if min_size.y <= 0.0 {
                    min_size.y = Self::DEFAULT_MIN_HEIGHT;
                }
            }

            min_size
        } else {
            KVector::new()
        }
    }

    pub fn apply_configured_node_scaling(node: &ElkNodeRef) {
        LayoutMetaDataService::get_instance();
        let scaling_factor = {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::SCALE_FACTOR)
        }
        .unwrap_or(1.0);

        if (scaling_factor - 1.0).abs() < f64::EPSILON {
            return;
        }

        {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            shape.set_dimensions(
                shape.width() * scaling_factor,
                shape.height() * scaling_factor,
            );
        }

        let node_labels: Vec<ElkLabelRef> = {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .labels()
                .iter()
                .cloned()
                .collect()
        };

        let ports: Vec<ElkPortRef> = {
            let mut node_mut = node.borrow_mut();
            node_mut.ports().iter().cloned().collect()
        };

        let mut port_labels: Vec<ElkLabelRef> = Vec::new();
        for port in &ports {
            let labels: Vec<ElkLabelRef> = {
                let mut port_mut = port.borrow_mut();
                port_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect()
            };
            port_labels.extend(labels);
        }

        for label in node_labels.iter().chain(port_labels.iter()) {
            let mut label_mut = label.borrow_mut();
            let shape = label_mut.shape();
            shape.set_location(shape.x() * scaling_factor, shape.y() * scaling_factor);
            shape.set_dimensions(
                shape.width() * scaling_factor,
                shape.height() * scaling_factor,
            );
            let props = shape.graph_element().properties_mut();
            if let Some(mut anchor) = props.get_property(CoreOptions::PORT_ANCHOR) {
                anchor.x *= scaling_factor;
                anchor.y *= scaling_factor;
                props.set_property(CoreOptions::PORT_ANCHOR, Some(anchor));
            }
        }

        for port in ports {
            let mut port_mut = port.borrow_mut();
            let shape = port_mut.connectable().shape();
            shape.set_location(shape.x() * scaling_factor, shape.y() * scaling_factor);
            shape.set_dimensions(
                shape.width() * scaling_factor,
                shape.height() * scaling_factor,
            );
            let props = shape.graph_element().properties_mut();
            if let Some(mut anchor) = props.get_property(CoreOptions::PORT_ANCHOR) {
                anchor.x *= scaling_factor;
                anchor.y *= scaling_factor;
                props.set_property(CoreOptions::PORT_ANCHOR, Some(anchor));
            }
        }
    }

    pub fn compute_child_area_dimensions(node: &ElkNodeRef) {
        LayoutMetaDataService::get_instance();
        let mut min_x: f64 = f64::INFINITY;
        let mut min_y: f64 = f64::INFINITY;
        let mut max_x: f64 = 0.0;
        let mut max_y: f64 = 0.0;

        let edge_labels: Vec<ElkLabelRef> = {
            let edges: Vec<ElkEdgeRef> = {
                let mut node_mut = node.borrow_mut();
                node_mut.contained_edges().iter().cloned().collect()
            };
            let mut labels = Vec::new();
            for edge in edges {
                let edge_labels: Vec<ElkLabelRef> = {
                    let mut edge_mut = edge.borrow_mut();
                    edge_mut.element().labels().iter().cloned().collect()
                };
                labels.extend(edge_labels);
            }
            labels
        };

        let node_labels: Vec<ElkLabelRef> = {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .labels()
                .iter()
                .cloned()
                .collect()
        };

        let children: Vec<ElkNodeRef> = {
            let mut node_mut = node.borrow_mut();
            node_mut.children().iter().cloned().collect()
        };

        for label in node_labels.iter().chain(edge_labels.iter()) {
            let mut label_mut = label.borrow_mut();
            let shape = label_mut.shape();
            let margins = shape
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::MARGINS)
                .unwrap_or_else(ElkMargin::new);
            min_x = min_x.min(shape.x() - margins.left);
            min_y = min_y.min(shape.y() - margins.top);
            max_x = max_x.max(shape.x() + shape.width() + margins.right);
            max_y = max_y.max(shape.y() + shape.height() + margins.bottom);
        }

        for child in &children {
            let mut child_mut = child.borrow_mut();
            let shape = child_mut.connectable().shape();
            let margins = shape
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::MARGINS)
                .unwrap_or_else(ElkMargin::new);
            min_x = min_x.min(shape.x() - margins.left);
            min_y = min_y.min(shape.y() - margins.top);
            max_x = max_x.max(shape.x() + shape.width() + margins.right);
            max_y = max_y.max(shape.y() + shape.height() + margins.bottom);
        }

        let edges: Vec<ElkEdgeRef> = {
            let mut node_mut = node.borrow_mut();
            node_mut.contained_edges().iter().cloned().collect()
        };
        for edge in edges {
            let sections: Vec<ElkEdgeSectionRef> = {
                let mut edge_mut = edge.borrow_mut();
                let list = edge_mut.sections();
                (0..list.len()).filter_map(|i| list.get(i)).collect()
            };
            for section in sections {
                let bend_points: Vec<ElkBendPointRef> = {
                    let mut section_mut = section.borrow_mut();
                    let s_x = section_mut.start_x();
                    let s_y = section_mut.start_y();
                    let e_x = section_mut.end_x();
                    let e_y = section_mut.end_y();
                    min_x = min_x.min(s_x);
                    min_x = min_x.min(e_x);
                    max_x = max_x.max(s_x);
                    max_x = max_x.max(e_x);
                    min_y = min_y.min(s_y);
                    min_y = min_y.min(e_y);
                    max_y = max_y.max(s_y);
                    max_y = max_y.max(e_y);
                    section_mut.bend_points().to_vec()
                };
                for bend_point in bend_points {
                    let bend = bend_point.borrow();
                    min_x = min_x.min(bend.x());
                    max_x = max_x.max(bend.x());
                    min_y = min_y.min(bend.y());
                    max_y = max_y.max(bend.y());
                }
            }
        }

        if !min_x.is_finite() || !min_y.is_finite() {
            min_x = 0.0;
            min_y = 0.0;
            max_x = 0.0;
            max_y = 0.0;
        }

        let width = max_x - min_x;
        let height = max_y - min_y;
        let mut node_mut = node.borrow_mut();
        node_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::CHILD_AREA_WIDTH, Some(width));
        node_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::CHILD_AREA_HEIGHT, Some(height));
    }
    fn translate_node_offset(parent: &ElkNodeRef, xoffset: f64, yoffset: f64) {
        let children: Vec<ElkNodeRef> = {
            let mut parent_mut = parent.borrow_mut();
            parent_mut.children().iter().cloned().collect()
        };
        for child in children {
            let mut child_mut = child.borrow_mut();
            let shape = child_mut.connectable().shape();
            let x = shape.x();
            let y = shape.y();
            shape.set_location(x + xoffset, y + yoffset);
        }

        let edges: Vec<ElkEdgeRef> = {
            let mut parent_mut = parent.borrow_mut();
            parent_mut.contained_edges().iter().cloned().collect()
        };
        for edge in edges {
            Self::translate_edge(&edge, xoffset, yoffset);
        }
    }

    fn translate_edge(edge: &ElkEdgeRef, xoffset: f64, yoffset: f64) {
        let sections: Vec<ElkEdgeSectionRef> = {
            let mut edge_mut = edge.borrow_mut();
            let list = edge_mut.sections();
            (0..list.len()).filter_map(|i| list.get(i)).collect()
        };
        for section in sections {
            Self::translate_section(&section, xoffset, yoffset);
        }

        let labels: Vec<ElkLabelRef> = {
            let mut edge_mut = edge.borrow_mut();
            edge_mut.element().labels().iter().cloned().collect()
        };
        for label in labels {
            let mut label_mut = label.borrow_mut();
            let shape = label_mut.shape();
            let x = shape.x();
            let y = shape.y();
            shape.set_location(x + xoffset, y + yoffset);
        }

        let junction_points = {
            let mut edge_mut = edge.borrow_mut();
            edge_mut
                .element()
                .properties_mut()
                .get_property(CoreOptions::JUNCTION_POINTS)
        };
        if let Some(mut junction_points) = junction_points {
            junction_points.offset(xoffset, yoffset);
            let mut edge_mut = edge.borrow_mut();
            edge_mut
                .element()
                .properties_mut()
                .set_property(CoreOptions::JUNCTION_POINTS, Some(junction_points));
        }
    }

    fn translate_section(section: &ElkEdgeSectionRef, xoffset: f64, yoffset: f64) {
        let bend_points: Vec<ElkBendPointRef> = {
            let mut section_mut = section.borrow_mut();
            let start_x = section_mut.start_x() + xoffset;
            let start_y = section_mut.start_y() + yoffset;
            let end_x = section_mut.end_x() + xoffset;
            let end_y = section_mut.end_y() + yoffset;
            section_mut.set_start_x(start_x);
            section_mut.set_start_y(start_y);
            section_mut.set_end_x(end_x);
            section_mut.set_end_y(end_y);
            section_mut.bend_points().to_vec()
        };

        for bend_point in bend_points {
            let mut bend_mut = bend_point.borrow_mut();
            let x = bend_mut.x() + xoffset;
            let y = bend_mut.y() + yoffset;
            bend_mut.set_x(x);
            bend_mut.set_y(y);
        }
    }

    fn translate_node_with_sizes(parent: &ElkNodeRef, new_size: &KVector, old_size: &KVector) {
        LayoutMetaDataService::get_instance();
        let content_alignment = {
            let mut parent_mut = parent.borrow_mut();
            parent_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::CONTENT_ALIGNMENT)
        }
        .unwrap_or_else(ContentAlignment::top_left);

        let mut x_translate = 0.0;
        let mut y_translate = 0.0;

        if new_size.x > old_size.x {
            if content_alignment.contains(&ContentAlignment::HCenter) {
                x_translate = (new_size.x - old_size.x) / 2.0;
            } else if content_alignment.contains(&ContentAlignment::HRight) {
                x_translate = new_size.x - old_size.x;
            }
        }

        if new_size.y > old_size.y {
            if content_alignment.contains(&ContentAlignment::VCenter) {
                y_translate = (new_size.y - old_size.y) / 2.0;
            } else if content_alignment.contains(&ContentAlignment::VBottom) {
                y_translate = new_size.y - old_size.y;
            }
        }

        Self::translate_node_offset(parent, x_translate, y_translate);
    }

    pub fn absolute_position(element: &ElkGraphElementRef) -> Option<KVector> {
        match element {
            ElkGraphElementRef::Node(node) => {
                let (x, y, parent) = {
                    let mut node_mut = node.borrow_mut();
                    let shape = node_mut.connectable().shape();
                    (shape.x(), shape.y(), node_mut.parent())
                };
                Some(Self::to_absolute(KVector::with_values(x, y), parent))
            }
            ElkGraphElementRef::Port(port) => {
                let (x, y, parent) = {
                    let mut port_mut = port.borrow_mut();
                    let shape = port_mut.connectable().shape();
                    (shape.x(), shape.y(), port_mut.parent())
                };
                Some(Self::to_absolute(KVector::with_values(x, y), parent))
            }
            ElkGraphElementRef::Edge(edge) => edge
                .borrow()
                .containing_node()
                .and_then(|node| Self::absolute_position(&ElkGraphElementRef::Node(node))),
            ElkGraphElementRef::Label(label) => {
                let (x, y, parent) = {
                    let mut label_mut = label.borrow_mut();
                    let shape = label_mut.shape();
                    (shape.x(), shape.y(), label_mut.parent())
                };
                let parent_pos = parent.and_then(|parent| Self::absolute_position(&parent))?;
                Some(KVector::with_values(parent_pos.x + x, parent_pos.y + y))
            }
        }
    }

    pub fn to_absolute(mut point: KVector, parent: Option<ElkNodeRef>) -> KVector {
        let mut current = parent;
        while let Some(node) = current {
            let (x, y, next) = {
                let mut node_mut = node.borrow_mut();
                let shape = node_mut.connectable().shape();
                (shape.x(), shape.y(), node_mut.parent())
            };
            point.add_values(x, y);
            current = next;
        }
        point
    }

    pub fn to_relative(mut point: KVector, parent: Option<ElkNodeRef>) -> KVector {
        let mut current = parent;
        while let Some(node) = current {
            let (x, y, next) = {
                let mut node_mut = node.borrow_mut();
                let shape = node_mut.connectable().shape();
                (shape.x(), shape.y(), node_mut.parent())
            };
            point.add_values(-x, -y);
            current = next;
        }
        point
    }

    pub fn create_vector_chain(edge_section: &ElkEdgeSectionRef) -> KVectorChain {
        let (start_x, start_y, end_x, end_y, bend_points) = {
            let mut section_mut = edge_section.borrow_mut();
            (
                section_mut.start_x(),
                section_mut.start_y(),
                section_mut.end_x(),
                section_mut.end_y(),
                section_mut.bend_points().clone(),
            )
        };

        let mut chain = KVectorChain::new();
        chain.add_vector(KVector::with_values(start_x, start_y));
        for bend_point in bend_points {
            let bend = bend_point.borrow();
            chain.add_vector(KVector::with_values(bend.x(), bend.y()));
        }
        chain.add_vector(KVector::with_values(end_x, end_y));
        chain
    }

    pub fn apply_vector_chain(vector_chain: &KVectorChain, section: &ElkEdgeSectionRef) {
        if vector_chain.size() < 2 {
            panic!("The vector chain must contain at least a source and a target point.");
        }

        let first_point = vector_chain.get(0);
        let last_point = vector_chain.get(vector_chain.size() - 1);
        let new_bend_count = vector_chain.size() - 2;

        let mut section_mut = section.borrow_mut();
        section_mut.set_start_x(first_point.x);
        section_mut.set_start_y(first_point.y);

        {
            let bend_points = section_mut.bend_points();
            if bend_points.len() < new_bend_count {
                let factory = ElkGraphFactory::instance();
                for _ in bend_points.len()..new_bend_count {
                    bend_points.push(factory.create_elk_bend_point());
                }
            } else if bend_points.len() > new_bend_count {
                bend_points.truncate(new_bend_count);
            }

            for (index, bend_point) in bend_points.iter().enumerate() {
                let point = vector_chain.get(index + 1);
                let mut bend_mut = bend_point.borrow_mut();
                bend_mut.set_x(point.x);
                bend_mut.set_y(point.y);
            }
        }

        section_mut.set_end_x(last_point.x);
        section_mut.set_end_y(last_point.y);
    }

    pub fn determine_junction_points(edge: &ElkEdgeRef) -> KVectorChain {
        let section_count = {
            let mut edge_mut = edge.borrow_mut();
            edge_mut.sections().len()
        };
        if section_count != 1 {
            panic!(
                "The edge needs to have exactly one edge section. Found: {}",
                section_count
            );
        }

        let mut junction_points = KVectorChain::new();
        let (source_shape, target_shape) = {
            let edge_borrow = edge.borrow();
            (
                edge_borrow.sources_ro().get(0),
                edge_borrow.targets_ro().get(0),
            )
        };

        if let Some(shape) = source_shape {
            if let Some(port) = ElkGraphUtil::connectable_shape_to_port(&shape) {
                let points = Self::determine_junction_points_for_port(edge, &port, false);
                junction_points.add_all(&points.to_array());
            }
        }

        if let Some(shape) = target_shape {
            if let Some(port) = ElkGraphUtil::connectable_shape_to_port(&shape) {
                let points = Self::determine_junction_points_for_port(edge, &port, true);
                junction_points.add_all(&points.to_array());
            }
        }

        junction_points
    }

    fn determine_junction_points_for_port(
        edge: &ElkEdgeRef,
        port: &ElkPortRef,
        reverse: bool,
    ) -> KVectorChain {
        let section = {
            let mut edge_mut = edge.borrow_mut();
            edge_mut
                .sections()
                .get(0)
                .expect("edge must have a section")
        };

        let section_points = Self::get_points(&section);
        let mut points_map: HashMap<usize, Vec<KVector>> = HashMap::new();
        let mut offset_map: HashMap<usize, KVector> = HashMap::new();
        let mut all_connected_sections: Vec<ElkEdgeSectionRef> = Vec::new();

        let incident_edges =
            ElkGraphUtil::all_incident_edges_for_shape(&ElkConnectableShapeRef::Port(port.clone()));
        for other_edge in incident_edges {
            let other_section_count = {
                let mut other_edge_mut = other_edge.borrow_mut();
                other_edge_mut.sections().len()
            };
            if other_section_count != 1 {
                panic!(
                    "The edge needs to have exactly one edge section. Found: {}",
                    other_section_count
                );
            }
            if Rc::ptr_eq(&other_edge, edge) {
                continue;
            }

            let other_section = {
                let mut other_edge_mut = other_edge.borrow_mut();
                other_edge_mut
                    .sections()
                    .get(0)
                    .expect("edge must have a section")
            };
            let key = Self::section_key(&other_section);
            points_map
                .entry(key)
                .or_insert_with(|| Self::get_points(&other_section));
            let other_points = points_map.get(&key).expect("other section points missing");

            let offset = if reverse {
                let mut offset = KVector::from_vector(&section_points[section_points.len() - 1]);
                offset.sub(&other_points[other_points.len() - 1]);
                offset
            } else {
                let mut offset = KVector::from_vector(&section_points[0]);
                offset.sub(&other_points[0]);
                offset
            };

            offset_map.insert(key, offset);
            all_connected_sections.push(other_section);
        }

        let mut junction_points = KVectorChain::new();
        if !all_connected_sections.is_empty() {
            let mut p1 = if reverse {
                section_points[section_points.len() - 1]
            } else {
                section_points[0]
            };

            for i in 1..section_points.len() {
                let p2 = if reverse {
                    section_points[section_points.len() - 1 - i]
                } else {
                    section_points[i]
                };

                let mut idx = 0;
                while idx < all_connected_sections.len() {
                    let other_section = &all_connected_sections[idx];
                    let key = Self::section_key(other_section);
                    let other_points = points_map.get(&key).expect("other section points missing");
                    if other_points.len() <= i {
                        all_connected_sections.remove(idx);
                        continue;
                    }

                    let other_index = if reverse {
                        other_points.len() - 1 - i
                    } else {
                        i
                    };
                    let mut p3 = KVector::from_vector(&other_points[other_index]);
                    if let Some(offset) = offset_map.get(&key) {
                        p3.add(offset);
                    }

                    if p2.x != p3.x || p2.y != p3.y {
                        let dx2 = p2.x - p1.x;
                        let dy2 = p2.y - p1.y;
                        let dx3 = p3.x - p1.x;
                        let dy3 = p3.y - p1.y;

                        if (dx3 * dy2) == (dy3 * dx2)
                            && dx2.signum() == dx3.signum()
                            && dy2.signum() == dy3.signum()
                        {
                            if dx2.abs() < dx3.abs() || dy2.abs() < dy3.abs() {
                                junction_points.add_vector(p2);
                            }
                        } else if i > 1 {
                            junction_points.add_vector(p1);
                        }

                        all_connected_sections.remove(idx);
                        continue;
                    }
                    idx += 1;
                }

                p1 = p2;
            }
        }

        junction_points
    }

    fn get_points(section: &ElkEdgeSectionRef) -> Vec<KVector> {
        let (start_x, start_y, end_x, end_y, bend_points) = {
            let mut section_mut = section.borrow_mut();
            (
                section_mut.start_x(),
                section_mut.start_y(),
                section_mut.end_x(),
                section_mut.end_y(),
                section_mut.bend_points().clone(),
            )
        };

        let mut points = Vec::with_capacity(bend_points.len() + 2);
        points.push(KVector::with_values(start_x, start_y));
        for bend_point in bend_points {
            let bend = bend_point.borrow();
            points.push(KVector::with_values(bend.x(), bend.y()));
        }
        points.push(KVector::with_values(end_x, end_y));

        let mut i = 1usize;
        while i < points.len().saturating_sub(1) {
            let p1 = points[i - 1];
            let p2 = points[i];
            let p3 = points[i + 1];

            if (p1.x == p2.x && p2.x == p3.x) || (p1.y == p2.y && p2.y == p3.y) {
                points.remove(i);
            } else {
                i += 1;
            }
        }

        points
    }

    pub fn get_labels_bounds_for_port(port: &ElkPortRef) -> ElkRectangle {
        let labels: Vec<ElkLabelRef> = {
            let mut port_mut = port.borrow_mut();
            port_mut
                .connectable()
                .shape()
                .graph_element()
                .labels()
                .iter()
                .cloned()
                .collect()
        };

        let mut bounds: Option<ElkRectangle> = None;
        for label in labels {
            let mut label_mut = label.borrow_mut();
            let shape = label_mut.shape();
            let current =
                ElkRectangle::with_values(shape.x(), shape.y(), shape.width(), shape.height());
            if let Some(existing) = bounds.as_mut() {
                existing.union(&current);
            } else {
                bounds = Some(current);
            }
        }

        bounds.unwrap_or_default()
    }

    pub fn compute_inside_part(
        label_position: &KVector,
        label_size: &KVector,
        port_size: &KVector,
        port_border_offset: f64,
        port_side: PortSide,
    ) -> f64 {
        match port_side {
            PortSide::North => {
                (label_size.y + label_position.y - (port_size.y + port_border_offset)).max(0.0)
            }
            PortSide::South => (-label_position.y - port_border_offset).max(0.0),
            PortSide::East => (-label_position.x - port_border_offset).max(0.0),
            PortSide::West => {
                (label_size.x + label_position.x - (port_size.x + port_border_offset)).max(0.0)
            }
            PortSide::Undefined => 0.0,
        }
    }

    pub fn configure_defaults_recursively(graph: &ElkNodeRef) {
        let children: Vec<ElkNodeRef> = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.children().iter().cloned().collect()
        };
        for child in &children {
            Self::configure_with_default_values(child);
            Self::configure_defaults_recursively(child);
        }

        let ports: Vec<ElkPortRef> = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.ports().iter().cloned().collect()
        };
        for port in &ports {
            Self::configure_with_default_values(port);
        }

        let edges: Vec<ElkEdgeRef> = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut.contained_edges().iter().cloned().collect()
        };
        for edge in &edges {
            Self::configure_with_default_values(edge);
        }
    }

    pub fn configure_with_default_values<A: ConfigureWithDefaultValuesArgs>(args: A) {
        args.configure();
    }

    fn configure_node_with_default_values(node: &ElkNodeRef) {
        LayoutMetaDataService::get_instance();
        let size_constraint = {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::NODE_SIZE_CONSTRAINTS)
        }
        .unwrap_or_else(SizeConstraint::fixed);

        if size_constraint == SizeConstraint::fixed() {
            let (width, height) = {
                let mut node_mut = node.borrow_mut();
                let shape = node_mut.connectable().shape();
                (shape.width(), shape.height())
            };
            if width == 0.0 && height == 0.0 {
                let mut node_mut = node.borrow_mut();
                let shape = node_mut.connectable().shape();
                shape.set_width(Self::DEFAULT_MIN_WIDTH * 4.0);
                shape.set_height(Self::DEFAULT_MIN_HEIGHT * 4.0);
            }
        }

        Self::ensure_label(&ElkGraphElementRef::Node(node.clone()));

        let placement = {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::NODE_LABELS_PLACEMENT)
        }
        .unwrap_or_else(NodeLabelPlacement::fixed);

        if placement == NodeLabelPlacement::fixed() {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .set_property(
                    CoreOptions::NODE_LABELS_PLACEMENT,
                    Some(NodeLabelPlacement::inside_center()),
                );
        }
    }

    fn configure_port_with_default_values(port: &ElkPortRef) {
        LayoutMetaDataService::get_instance();
        let (width, height) = {
            let mut port_mut = port.borrow_mut();
            let shape = port_mut.connectable().shape();
            (shape.width(), shape.height())
        };

        if width == 0.0 && height == 0.0 {
            let mut port_mut = port.borrow_mut();
            let shape = port_mut.connectable().shape();
            shape.set_width(Self::DEFAULT_MIN_WIDTH / 4.0);
            shape.set_height(Self::DEFAULT_MIN_HEIGHT / 4.0);
        }

        Self::ensure_label(&ElkGraphElementRef::Port(port.clone()));
    }

    fn configure_edge_with_default_values(edge: &ElkEdgeRef) {
        LayoutMetaDataService::get_instance();
        let has_placement = {
            let mut edge_mut = edge.borrow_mut();
            edge_mut
                .element()
                .properties()
                .has_property(CoreOptions::EDGE_LABELS_PLACEMENT)
        };
        if !has_placement {
            let mut edge_mut = edge.borrow_mut();
            edge_mut.element().properties_mut().set_property(
                CoreOptions::EDGE_LABELS_PLACEMENT,
                Some(EdgeLabelPlacement::Center),
            );
        }
    }

    fn ensure_label(element: &ElkGraphElementRef) {
        let (has_labels, identifier) = match element {
            ElkGraphElementRef::Node(node) => {
                let mut node_mut = node.borrow_mut();
                let graph_element = node_mut.connectable().shape().graph_element();
                let has_labels = !graph_element.labels().is_empty();
                let identifier = graph_element.identifier().map(|value| value.to_string());
                (has_labels, identifier)
            }
            ElkGraphElementRef::Port(port) => {
                let mut port_mut = port.borrow_mut();
                let graph_element = port_mut.connectable().shape().graph_element();
                let has_labels = !graph_element.labels().is_empty();
                let identifier = graph_element.identifier().map(|value| value.to_string());
                (has_labels, identifier)
            }
            ElkGraphElementRef::Edge(edge) => {
                let mut edge_mut = edge.borrow_mut();
                let graph_element = edge_mut.element();
                let has_labels = !graph_element.labels().is_empty();
                let identifier = graph_element.identifier().map(|value| value.to_string());
                (has_labels, identifier)
            }
            ElkGraphElementRef::Label(_) => return,
        };

        if has_labels {
            return;
        }
        if let Some(identifier) = identifier {
            if !identifier.is_empty() {
                let label = ElkGraphUtil::create_label(Some(element.clone()));
                label.borrow_mut().set_text(identifier);
            }
        }
    }

    pub fn apply_visitors(graph: &ElkNodeRef, visitors: &mut [&mut dyn IGraphElementVisitor]) {
        let root = ElkGraphElementRef::Node(graph.clone());
        Self::apply_visitors_to_element(&root, visitors);
    }

    pub fn apply_visitors_with_validation(
        graph: &ElkNodeRef,
        visitors: &mut [&mut dyn IGraphElementVisitor],
    ) -> Result<(), GraphValidationException> {
        Self::apply_visitors(graph, visitors);

        let mut all_issues: Vec<GraphIssue> = Vec::new();
        for visitor in visitors.iter() {
            if let Some(issues) = visitor.issues() {
                if !issues.is_empty() {
                    all_issues.extend(issues.iter().cloned());
                }
            }
        }

        if all_issues
            .iter()
            .any(|issue| issue.severity() == Severity::Error)
        {
            let mut message = String::new();
            for issue in &all_issues {
                if !message.is_empty() {
                    message.push('\n');
                }
                message.push_str(&format!("{}: {}", issue.severity(), issue.message()));
                message.push_str("\n\tat ");
                if let Some(element) = issue.element() {
                    Self::print_element_path(element, &mut message);
                } else {
                    message.push_str("Root");
                }
            }
            return Err(GraphValidationException::new(message, all_issues));
        }

        Ok(())
    }

    fn apply_visitors_to_element(
        element: &ElkGraphElementRef,
        visitors: &mut [&mut dyn IGraphElementVisitor],
    ) {
        for visitor in visitors.iter_mut() {
            visitor.visit(element);
        }

        let labels = Self::labels_for_element(element);
        for label in labels {
            Self::apply_visitors_to_element(&ElkGraphElementRef::Label(label), visitors);
        }

        if let ElkGraphElementRef::Node(node) = element {
            let ports: Vec<ElkPortRef> = {
                let mut node_mut = node.borrow_mut();
                node_mut.ports().iter().cloned().collect()
            };
            for port in ports {
                Self::apply_visitors_to_element(&ElkGraphElementRef::Port(port), visitors);
            }

            let children: Vec<ElkNodeRef> = {
                let mut node_mut = node.borrow_mut();
                node_mut.children().iter().cloned().collect()
            };
            for child in children {
                Self::apply_visitors_to_element(&ElkGraphElementRef::Node(child), visitors);
            }

            let edges: Vec<ElkEdgeRef> = {
                let mut node_mut = node.borrow_mut();
                node_mut.contained_edges().iter().cloned().collect()
            };
            for edge in edges {
                Self::apply_visitors_to_element(&ElkGraphElementRef::Edge(edge), visitors);
            }
        }
    }

    fn labels_for_element(element: &ElkGraphElementRef) -> Vec<ElkLabelRef> {
        match element {
            ElkGraphElementRef::Node(node) => {
                let mut node_mut = node.borrow_mut();
                node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect()
            }
            ElkGraphElementRef::Port(port) => {
                let mut port_mut = port.borrow_mut();
                port_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect()
            }
            ElkGraphElementRef::Edge(edge) => {
                let mut edge_mut = edge.borrow_mut();
                edge_mut.element().labels().iter().cloned().collect()
            }
            ElkGraphElementRef::Label(label) => {
                let mut label_mut = label.borrow_mut();
                label_mut
                    .shape()
                    .graph_element()
                    .labels()
                    .iter()
                    .cloned()
                    .collect()
            }
        }
    }

    pub fn print_element_path(element: &ElkGraphElementRef, builder: &mut String) {
        if let Some(parent) = Self::element_container(element) {
            Self::print_element_path(&parent, builder);
            builder.push_str(" > ");
        } else {
            builder.push_str("Root ");
        }

        builder.push_str(Self::element_class_name(element));

        if let Some(identifier) = Self::element_identifier(element) {
            if !identifier.is_empty() {
                builder.push(' ');
                builder.push_str(&identifier);
                return;
            }
        }

        if let ElkGraphElementRef::Label(label) = element {
            let text = label.borrow().text().to_string();
            if !text.is_empty() {
                builder.push(' ');
                builder.push_str(&text);
                return;
            }
        }

        if let Some(text) = Self::first_label_text(element) {
            builder.push(' ');
            builder.push_str(&text);
            return;
        }

        if let ElkGraphElementRef::Edge(edge) = element {
            let (sources, targets, connected) = {
                let edge_ref = edge.borrow();
                (
                    edge_ref.sources_ro().iter().cloned().collect::<Vec<_>>(),
                    edge_ref.targets_ro().iter().cloned().collect::<Vec<_>>(),
                    edge_ref.is_connected(),
                )
            };

            if connected {
                builder.push_str(" (");
                for (index, source) in sources.into_iter().enumerate() {
                    if index > 0 {
                        builder.push_str(", ");
                    }
                    Self::print_element_path(&ElkGraphElementRef::from(source), builder);
                }
                builder.push_str(" -> ");
                for (index, target) in targets.into_iter().enumerate() {
                    if index > 0 {
                        builder.push_str(", ");
                    }
                    Self::print_element_path(&ElkGraphElementRef::from(target), builder);
                }
                builder.push(')');
            }
        }
    }

    fn element_container(element: &ElkGraphElementRef) -> Option<ElkGraphElementRef> {
        match element {
            ElkGraphElementRef::Node(node) => node.borrow().parent().map(ElkGraphElementRef::Node),
            ElkGraphElementRef::Port(port) => port.borrow().parent().map(ElkGraphElementRef::Node),
            ElkGraphElementRef::Edge(edge) => edge
                .borrow()
                .containing_node()
                .map(ElkGraphElementRef::Node),
            ElkGraphElementRef::Label(label) => label.borrow().parent(),
        }
    }

    fn element_class_name(element: &ElkGraphElementRef) -> &'static str {
        match element {
            ElkGraphElementRef::Node(_) => "Node",
            ElkGraphElementRef::Edge(_) => "Edge",
            ElkGraphElementRef::Port(_) => "Port",
            ElkGraphElementRef::Label(_) => "Label",
        }
    }

    fn element_identifier(element: &ElkGraphElementRef) -> Option<String> {
        match element {
            ElkGraphElementRef::Node(node) => {
                let mut node_mut = node.borrow_mut();
                node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(|value| value.to_string())
            }
            ElkGraphElementRef::Port(port) => {
                let mut port_mut = port.borrow_mut();
                port_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(|value| value.to_string())
            }
            ElkGraphElementRef::Edge(edge) => {
                let mut edge_mut = edge.borrow_mut();
                edge_mut
                    .element()
                    .identifier()
                    .map(|value| value.to_string())
            }
            ElkGraphElementRef::Label(label) => {
                let mut label_mut = label.borrow_mut();
                label_mut
                    .shape()
                    .graph_element()
                    .identifier()
                    .map(|value| value.to_string())
            }
        }
    }

    fn first_label_text(element: &ElkGraphElementRef) -> Option<String> {
        let labels = Self::labels_for_element(element);
        for label in labels {
            let text = label.borrow().text().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
        None
    }

    pub fn debug_folder_path(subfolders: &[&str]) -> Option<String> {
        let user_home = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE"))?;
        let mut path = user_home.to_string_lossy().to_string();

        if !path.ends_with(MAIN_SEPARATOR) {
            path.push(MAIN_SEPARATOR);
        }

        path.push_str("elk");
        path.push(MAIN_SEPARATOR);

        for subfolder in subfolders {
            path.push_str(subfolder);
            path.push(MAIN_SEPARATOR);
        }

        Some(path)
    }

    pub fn to_safe_path_name(name: &str) -> String {
        let mut output = String::with_capacity(name.len());
        for ch in name.chars() {
            if ch.is_whitespace() {
                output.push('_');
            } else if ch.is_ascii_alphanumeric() || ch == '_' {
                output.push(ch);
            } else {
                output.push('-');
            }
        }
        output
    }

    pub fn create_identifier(element: &ElkGraphElementRef) {
        let id = match element {
            ElkGraphElementRef::Node(node) => Rc::as_ptr(node) as usize,
            ElkGraphElementRef::Edge(edge) => Rc::as_ptr(edge) as usize,
            ElkGraphElementRef::Port(port) => Rc::as_ptr(port) as usize,
            ElkGraphElementRef::Label(label) => Rc::as_ptr(label) as usize,
        };

        let identifier = id.to_string();
        match element {
            ElkGraphElementRef::Node(node) => {
                let mut node_mut = node.borrow_mut();
                node_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .set_identifier(Some(identifier));
            }
            ElkGraphElementRef::Edge(edge) => {
                let mut edge_mut = edge.borrow_mut();
                edge_mut.element().set_identifier(Some(identifier));
            }
            ElkGraphElementRef::Port(port) => {
                let mut port_mut = port.borrow_mut();
                port_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .set_identifier(Some(identifier));
            }
            ElkGraphElementRef::Label(label) => {
                let mut label_mut = label.borrow_mut();
                label_mut
                    .shape()
                    .graph_element()
                    .set_identifier(Some(identifier));
            }
        }
    }

    fn section_key(section: &ElkEdgeSectionRef) -> usize {
        Rc::as_ptr(section) as usize
    }
}

impl TranslateArgs for (&ElkNodeRef, f64, f64) {
    fn translate(self) {
        ElkUtil::translate_node_offset(self.0, self.1, self.2);
    }
}

impl TranslateArgs for (&ElkEdgeRef, f64, f64) {
    fn translate(self) {
        ElkUtil::translate_edge(self.0, self.1, self.2);
    }
}

impl TranslateArgs for (&ElkEdgeSectionRef, f64, f64) {
    fn translate(self) {
        ElkUtil::translate_section(self.0, self.1, self.2);
    }
}

impl TranslateArgs for (&ElkNodeRef, &KVector, &KVector) {
    fn translate(self) {
        ElkUtil::translate_node_with_sizes(self.0, self.1, self.2);
    }
}

impl ConfigureWithDefaultValuesArgs for &ElkNodeRef {
    fn configure(self) {
        ElkUtil::configure_node_with_default_values(self);
    }
}

impl ConfigureWithDefaultValuesArgs for &ElkPortRef {
    fn configure(self) {
        ElkUtil::configure_port_with_default_values(self);
    }
}

impl ConfigureWithDefaultValuesArgs for &ElkEdgeRef {
    fn configure(self) {
        ElkUtil::configure_edge_with_default_values(self);
    }
}
