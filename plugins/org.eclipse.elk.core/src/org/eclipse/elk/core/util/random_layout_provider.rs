use std::time::{SystemTime, UNIX_EPOCH};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdgeRef, ElkGraphFactory, ElkNodeRef,
};

use crate::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use crate::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use crate::org::eclipse::elk::core::math::ElkPadding;
use crate::org::eclipse::elk::core::options::RandomLayouterOptions;
use crate::org::eclipse::elk::core::util::{ElkUtil, IElkProgressMonitor, Random};

#[derive(Clone, Default)]
pub struct RandomLayoutProvider;

impl RandomLayoutProvider {
    pub fn new() -> Self {
        RandomLayoutProvider
    }
}

impl IGraphLayoutEngine for RandomLayoutProvider {
    fn layout(&mut self, layout_graph: &ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Random Layout", 1.0);

        let has_children = {
            let mut layout_mut = layout_graph.borrow_mut();
            !layout_mut.children().is_empty()
        };
        if !has_children {
            progress_monitor.done();
            return;
        }

        let (random_seed, aspect_ratio, spacing, padding) = with_node_properties_mut(layout_graph, |props| {
            let seed = props.get_property(RandomLayouterOptions::RANDOM_SEED).unwrap_or(0);
            let aspect_ratio = props.get_property(RandomLayouterOptions::ASPECT_RATIO).unwrap_or(1.0);
            let spacing = props.get_property(RandomLayouterOptions::SPACING_NODE_NODE).unwrap_or(0.0);
            let padding = props
                .get_property(RandomLayouterOptions::PADDING)
                .unwrap_or_else(ElkPadding::new);
            (seed, aspect_ratio, spacing, padding)
        });

        let seed = if random_seed != 0 {
            random_seed as u64
        } else {
            seed_from_time()
        };
        let mut random = Random::new(seed);

        randomize(layout_graph, &mut random, aspect_ratio, spacing, &padding);

        progress_monitor.done();
    }
}

impl AbstractLayoutProvider for RandomLayoutProvider {}

fn randomize(
    parent: &ElkNodeRef,
    random: &mut Random,
    aspect_ratio: f64,
    spacing: f64,
    padding: &ElkPadding,
) {
    let children: Vec<ElkNodeRef> = {
        let mut parent_mut = parent.borrow_mut();
        parent_mut.children().iter().cloned().collect()
    };
    if children.is_empty() {
        return;
    }

    let mut nodes_area: f64 = 0.0;
    let mut max_width: f64 = 0.0;
    let mut max_height: f64 = 0.0;
    let mut m: usize = 1;

    for node in &children {
        m += ElkGraphUtil::all_outgoing_edges(node).len();
        let (width, height) = node_size(node);
        max_width = max_width.max(width);
        max_height = max_height.max(height);
        nodes_area += width * height;
    }

    let n = children.len() as f64;
    let draw_area = nodes_area + 2.0 * spacing * spacing * (m as f64) * n;
    let area_sqrt = draw_area.sqrt();
    let draw_width = (area_sqrt * aspect_ratio).max(max_width);
    let draw_height = (area_sqrt / aspect_ratio).max(max_height);

    for node in &children {
        let (width, height) = node_size(node);
        let x = padding.left + random.next_double() * (draw_width - width);
        let y = padding.left + random.next_double() * (draw_height - height);
        set_node_location(node, x, y);
    }

    let mut total_width = draw_width + padding.left + padding.right;
    let mut total_height = draw_height + padding.top + padding.bottom;

    for source in &children {
        for edge in ElkGraphUtil::all_outgoing_edges(source) {
            if !edge.borrow().is_hierarchical() {
                randomize_edge(&edge, random, total_width, total_height);
            }
        }
    }

    total_width += padding.left + padding.right;
    total_height += padding.top + padding.bottom;
    ElkUtil::resize_node_with(parent, total_width, total_height, false, true);
}

const MAX_BENDS: i32 = 5;
const RAND_FACT: f64 = 0.2;

fn randomize_edge(edge: &ElkEdgeRef, random: &mut Random, draw_width: f64, draw_height: f64) {
    let source_shape = {
        let edge_borrow = edge.borrow();
        edge_borrow.sources_ro().get(0)
    };
    let Some(source_shape) = source_shape else {
        return;
    };

    let (source_x, source_y, source_width, source_height) = connectable_center(&source_shape);

    let target_shape = {
        let edge_borrow = edge.borrow();
        edge_borrow.sources_ro().get(0)
    };
    let Some(target_shape) = target_shape else {
        return;
    };

    let (target_x, target_y, target_width, target_height) = connectable_center(&target_shape);

    let section = ensure_single_section(edge);
    let Some(section) = section else {
        return;
    };

    let mut source_px = target_x;
    if target_x > source_x + source_width {
        source_px = source_x + source_width;
    } else if target_x < source_x - source_width {
        source_px = source_x - source_width;
    }

    let mut source_py = target_y;
    if target_y > source_y + source_height {
        source_py = source_y + source_height;
    } else if target_y < source_y - source_height {
        source_py = source_y - source_height;
    }

    if source_px > source_x - source_width
        && source_px < source_x + source_width
        && source_py > source_y - source_height
        && source_py < source_y + source_height
    {
        source_px = source_x + source_width;
    }

    let mut target_px = source_x;
    if source_x > target_x + target_width {
        target_px = target_x + target_width;
    } else if source_x < target_x - target_width {
        target_px = target_x - target_width;
    }

    let mut target_py = source_y;
    if source_y > target_y + target_height {
        target_py = target_y + target_height;
    } else if source_y < target_y - target_height {
        target_py = target_y - target_height;
    }

    if target_px > target_x - target_width
        && target_px < target_x + target_width
        && target_py > target_y - target_height
        && target_py < target_y + target_height
    {
        target_py = target_y + target_height;
    }

    {
        let mut section_mut = section.borrow_mut();
        section_mut.set_start_x(source_px);
        section_mut.set_start_y(source_py);
        section_mut.set_end_x(target_px);
        section_mut.set_end_y(target_py);

        let bend_points = section_mut.bend_points();
        bend_points.clear();

        let mut bends_num = random.next_int(MAX_BENDS);
        if source_shape.ptr_eq(&target_shape) {
            bends_num += 1;
        }
        let xdiff = target_px - source_px;
        let ydiff = target_py - source_py;
        let total_dist = (xdiff * xdiff + ydiff * ydiff).sqrt();
        let max_rand = total_dist * RAND_FACT;
        let xincr = xdiff / ((bends_num + 1) as f64);
        let yincr = ydiff / ((bends_num + 1) as f64);
        let mut x = source_px;
        let mut y = source_py;

        for _ in 0..bends_num {
            x += xincr;
            y += yincr;
            let mut randx = x + random.next_float() * max_rand - max_rand / 2.0;
            if randx < 0.0 {
                randx = 1.0;
            } else if randx > draw_width {
                randx = draw_width - 1.0;
            }
            let mut randy = y + random.next_float() * max_rand - max_rand / 2.0;
            if randy < 0.0 {
                randy = 1.0;
            } else if randy > draw_height {
                randy = draw_height - 1.0;
            }

            let bend_point = ElkGraphFactory::instance().create_elk_bend_point();
            {
                let mut bend_mut = bend_point.borrow_mut();
                bend_mut.set_x(randx);
                bend_mut.set_y(randy);
            }
            bend_points.push(bend_point);
        }
    }
}

fn ensure_single_section(edge: &ElkEdgeRef) -> Option<org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeSectionRef> {
    let mut edge_mut = edge.borrow_mut();
    let sections = edge_mut.sections();
    if sections.is_empty() {
        let section = ElkGraphFactory::instance().create_elk_edge_section();
        sections.add(section.clone());
        return Some(section);
    }
    if sections.len() > 1 {
        sections.retain_last();
    }
    sections.get(0)
}

fn connectable_center(shape: &ElkConnectableShapeRef) -> (f64, f64, f64, f64) {
    let (x, y, width, height) = match shape {
        ElkConnectableShapeRef::Node(node) => {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            (shape.x(), shape.y(), shape.width(), shape.height())
        }
        ElkConnectableShapeRef::Port(port) => {
            let mut port_mut = port.borrow_mut();
            let shape = port_mut.connectable().shape();
            (shape.x(), shape.y(), shape.width(), shape.height())
        }
    };

    let mut cx = x;
    let mut cy = y;
    if let ElkConnectableShapeRef::Port(port) = shape {
        if let Some(parent) = port.borrow().parent() {
            let parent_x = {
                let mut parent_mut = parent.borrow_mut();
                parent_mut.connectable().shape().x()
            };
            cx += parent_x;
            cx += parent_x;
        }
    }

    let half_w = width / 2.0;
    let half_h = height / 2.0;
    cx += half_w;
    cy += half_h;

    (cx, cy, half_w, half_h)
}

fn node_size(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
}

fn set_node_location(node: &ElkNodeRef, x: f64, y: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_location(x, y);
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

fn seed_from_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0)
}
