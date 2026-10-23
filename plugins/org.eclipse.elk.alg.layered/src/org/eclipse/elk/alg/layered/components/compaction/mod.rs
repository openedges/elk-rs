// Copyright (c) 2016 Kiel University and others.
// SPDX-License-Identifier: EPL-2.0 OR GPL-3.0-or-later
// Port of the layered ComponentsCompactor, using the shared compaction kernel.

mod packing;

use super::components_processor::{collect_component_nodes, offset_graph};

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraphRef, LNode, LNodeRef, LPort, LPortRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::{
    point::Point, rectilinear_convex_hull::RectilinearConvexHull,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkRectangle, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

struct Extension {
    rectangle: ElkRectangle,
    placeholder: ElkRectangle,
    side: PortSide,
}

struct Component {
    hull: Vec<ElkRectangle>,
    sides: EnumSet<PortSide>,
    extensions: Vec<Extension>,
    external_edges: Vec<LEdgeRef>,
}

struct Bounds {
    min: KVector,
    max: KVector,
}

impl Bounds {
    fn new() -> Self {
        Self {
            min: KVector::with_values(f64::INFINITY, f64::INFINITY),
            max: KVector::with_values(f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }
    fn include(&mut self, point: KVector) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
    }
    fn external_position(&self, mut point: KVector, side: PortSide) -> KVector {
        match side {
            PortSide::North => point.y = point.y.min(self.min.y),
            PortSide::East => point.x = point.x.max(self.max.x),
            PortSide::South => point.y = point.y.max(self.max.y),
            PortSide::West => point.x = point.x.min(self.min.x),
            PortSide::Undefined => {}
        }
        point
    }
}

struct HullPoints {
    points: Vec<Point>,
    bounds: Bounds,
}

impl HullPoints {
    fn new() -> Self {
        Self {
            points: Vec::new(),
            bounds: Bounds::new(),
        }
    }
    fn add(&mut self, point: KVector) {
        self.bounds.include(point);
        self.points.push(Point::new(point.x, point.y));
    }
    fn rectangle(&mut self, rect: ElkRectangle) {
        self.add(rect.get_top_left());
        self.add(rect.get_bottom_left());
        self.add(rect.get_top_right());
        self.add(rect.get_bottom_right());
    }
}

pub(super) fn compact(graphs: &[LGraphRef], spacing: f64) -> (KVector, KVector) {
    let mut frame = Bounds::new();
    for graph in graphs {
        for node in collect_component_nodes(graph) {
            let rect = node_bounds(&node);
            frame.include(rect.get_top_left());
            frame.include(rect.get_bottom_right());
        }
    }
    let components: Vec<_> = graphs
        .iter()
        .map(|graph| transform(graph, &frame))
        .collect();
    let (offsets, mut size) = packing::pack(&components, spacing);
    for (graph, offset) in graphs.iter().zip(offsets) {
        offset_graph(graph, offset.x, offset.y);
        for node in collect_component_nodes(graph) {
            let mut node = node.lock();
            if node.node_type() == NodeType::ExternalPort {
                let side = node.ext_port_side();
                let position = frame.external_position(*node.shape().position_ref(), side);
                *node.shape().position() = position;
            }
        }
    }
    // Only the coordinate perpendicular to an external edge segment contributes.
    let mut offset = KVector::new();
    for component in &components {
        for edge in &component.external_edges {
            let (source, target, mut points) = edge_path(edge);
            points.insert(
                0,
                source
                    .lock()
                    .absolute_anchor()
                    .expect("component port belongs to a node"),
            );
            points.push(
                target
                    .lock()
                    .absolute_anchor()
                    .expect("component port belongs to a node"),
            );
            for segment in points.windows(2) {
                if (segment[0].x - segment[1].x).abs() <= 0.0001 {
                    offset.x = offset.x.min(segment[0].x);
                    size.x = size.x.max(segment[0].x);
                } else if (segment[0].y - segment[1].y).abs() <= 0.0001 {
                    offset.y = offset.y.min(segment[0].y);
                    size.y = size.y.max(segment[0].y);
                }
            }
        }
    }
    offset.x = -offset.x;
    offset.y = -offset.y;
    size.x += offset.x;
    size.y += offset.y;
    (size, offset)
}

fn node_bounds(node: &LNodeRef) -> ElkRectangle {
    let mut node = node.lock();
    let pos = *node.shape().position_ref();
    let size = *node.shape().size_ref();
    let margin = node.margin();
    ElkRectangle::with_values(
        pos.x - margin.left,
        pos.y - margin.top,
        size.x + margin.left + margin.right,
        size.y + margin.top + margin.bottom,
    )
}

fn edge_path(edge: &LEdgeRef) -> (LPortRef, LPortRef, Vec<KVector>) {
    let mut edge = edge.lock();
    (
        edge.source().unwrap(),
        edge.target().unwrap(),
        edge.bend_points().iter().copied().collect(),
    )
}

fn external(edge: &LEdgeRef) -> bool {
    let (source, target) = {
        let edge = edge.lock();
        (edge.source().unwrap(), edge.target().unwrap())
    };
    let source = source.lock().node().unwrap();
    let target = target.lock().node().unwrap();
    // Release the source lock before checking the target of a self-loop.
    let source_external = source.lock().node_type() == NodeType::ExternalPort;
    source_external || target.lock().node_type() == NodeType::ExternalPort
}

fn margin_anchor(port: &LPortRef) -> KVector {
    let (mut position, side, node) = {
        let port = port.lock();
        (
            port.absolute_anchor()
                .expect("component port belongs to a node"),
            port.side(),
            port.node().unwrap(),
        )
    };
    let mut node = node.lock();
    let margin = node.margin();
    match side {
        PortSide::North => position.y -= margin.top,
        PortSide::East => position.x += margin.right,
        PortSide::South => position.y += margin.bottom,
        PortSide::West => position.x -= margin.left,
        PortSide::Undefined => {}
    }
    position
}

fn side_index(side: PortSide) -> usize {
    match side {
        PortSide::North => 0,
        PortSide::East => 1,
        PortSide::South => 2,
        PortSide::West => 3,
        PortSide::Undefined => unreachable!("external port has a side"),
    }
}

fn transform(graph: &LGraphRef, frame: &Bounds) -> Component {
    let mut nodes = collect_component_nodes(graph).clone();
    if !nodes
        .iter()
        .any(|node| node.lock().node_type() == NodeType::Normal)
    {
        create_dummy(graph, &nodes[0]);
        nodes = collect_component_nodes(graph).clone();
    }
    let mut hull = HullPoints::new();
    for node in &nodes {
        if node.lock().node_type() == NodeType::ExternalPort {
            continue;
        }
        hull.rectangle(node_bounds(node));
        let edges = node.lock().outgoing_edges();
        for edge in edges {
            if !external(&edge) {
                for point in edge.lock().bend_points().iter() {
                    hull.add(*point);
                }
            }
        }
    }
    let mut min = [f64::INFINITY; 4];
    let mut max = [f64::NEG_INFINITY; 4];
    let mut extent = [f64::NEG_INFINITY; 4];
    let mut external_edges = Vec::new();
    for node in &nodes {
        let edges = node.lock().outgoing_edges();
        for edge in edges {
            if !external(&edge) {
                continue;
            }
            let (source, target, mut points) = edge_path(&edge);
            let source_node = source.lock().node().unwrap();
            let source_external = source_node.lock().node_type() == NodeType::ExternalPort;
            let external_node = if source_external {
                source_node
            } else {
                target.lock().node().unwrap()
            };
            let side = external_node.lock().ext_port_side();
            let first = if source_external {
                frame.external_position(
                    source
                        .lock()
                        .absolute_anchor()
                        .expect("component port belongs to a node"),
                    side,
                )
            } else {
                margin_anchor(&source)
            };
            let last = if source_external {
                margin_anchor(&target)
            } else {
                frame.external_position(
                    target
                        .lock()
                        .absolute_anchor()
                        .expect("component port belongs to a node"),
                    side,
                )
            };
            points.insert(0, first);
            points.push(last);
            let thickness = edge.lock().edge_thickness().max(1.0);
            for (index, segment) in points.windows(2).enumerate() {
                let a = segment[0];
                let b = segment[1];
                let rect = ElkRectangle::with_values(
                    a.x.min(b.x) - thickness / 2.0,
                    a.y.min(b.y) - thickness / 2.0,
                    (a.x - b.x).abs() + thickness,
                    (a.y - b.y).abs() + thickness,
                );
                let outer = if source_external {
                    index == 0
                } else {
                    index == points.len() - 2
                };
                if outer {
                    let i = side_index(side);
                    let (low, length) = if matches!(side, PortSide::West | PortSide::East) {
                        (rect.y, rect.height)
                    } else {
                        (rect.x, rect.width)
                    };
                    min[i] = min[i].min(low);
                    max[i] = max[i].max(low + length);
                } else {
                    hull.rectangle(rect);
                }
            }
            let mut external_node = external_node.lock();
            let margin = external_node.margin();
            let value = match side {
                PortSide::West => margin.right,
                PortSide::East => margin.left,
                PortSide::North => margin.bottom,
                PortSide::South => margin.top,
                PortSide::Undefined => unreachable!(),
            };
            let i = side_index(side);
            extent[i] = extent[i].max(value);
            external_edges.push(edge);
        }
    }
    let sides = graph
        .lock()
        .get_property(InternalProperties::EXT_PORT_CONNECTIONS)
        .unwrap_or_else(EnumSet::none_of);
    let mut extensions = Vec::new();
    for &side in sides.iter() {
        let i = side_index(side);
        let lo = min[i];
        let length = max[i] - lo;
        let width = extent[i];
        let (rectangle, placeholder) = match side {
            PortSide::West => (
                ElkRectangle::with_values(frame.min.x, lo, hull.bounds.min.x - frame.min.x, length),
                ElkRectangle::with_values(frame.min.x, lo, width, length),
            ),
            PortSide::East => (
                ElkRectangle::with_values(
                    hull.bounds.max.x,
                    lo,
                    frame.max.x - hull.bounds.max.x,
                    length,
                ),
                ElkRectangle::with_values(frame.max.x - width, lo, width, length),
            ),
            PortSide::North => (
                ElkRectangle::with_values(lo, frame.min.y, length, hull.bounds.min.y - frame.min.y),
                ElkRectangle::with_values(lo, frame.min.y, length, width),
            ),
            PortSide::South => (
                ElkRectangle::with_values(
                    lo,
                    hull.bounds.max.y,
                    length,
                    frame.max.y - hull.bounds.max.y,
                ),
                ElkRectangle::with_values(lo, frame.max.y - width, length, width),
            ),
            PortSide::Undefined => unreachable!(),
        };
        match side {
            PortSide::West => {
                hull.add(rectangle.get_top_right());
                hull.add(rectangle.get_bottom_right());
            }
            PortSide::East => {
                hull.add(rectangle.get_top_left());
                hull.add(rectangle.get_bottom_left());
            }
            PortSide::North => {
                hull.add(rectangle.get_bottom_left());
                hull.add(rectangle.get_bottom_right());
            }
            PortSide::South => {
                hull.add(rectangle.get_top_left());
                hull.add(rectangle.get_top_right());
            }
            PortSide::Undefined => unreachable!(),
        }
        extensions.push(Extension {
            rectangle,
            placeholder,
            side,
        });
    }
    Component {
        hull: RectilinearConvexHull::of(hull.points).split_into_rectangles(),
        sides,
        extensions,
        external_edges,
    }
}

fn create_dummy(graph: &LGraphRef, external: &LNodeRef) {
    let (mut position, size, side, port) = {
        let mut external = external.lock();
        (
            *external.shape().position_ref(),
            *external.shape().size_ref(),
            external.ext_port_side(),
            external.ports()[0].clone(),
        )
    };
    match side {
        PortSide::West => position.x += 2.0,
        PortSide::East => position.x -= 2.0,
        PortSide::North => position.y += 2.0,
        PortSide::South => position.y -= 2.0,
        PortSide::Undefined => {}
    }
    let dummy = LNode::new(graph);
    {
        let mut dummy = dummy.lock();
        *dummy.shape().position() = position;
        *dummy.shape().size() = KVector::with_values(size.x.max(1.0), size.y.max(1.0));
    }
    graph.lock().layerless_nodes_mut().push(dummy.clone());
    let dummy_port = LPort::new();
    LPort::set_node(&dummy_port, Some(dummy));
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(port.clone()));
    LEdge::set_target(&edge, Some(dummy_port.clone()));
    let mut port = port.lock();
    let mut dummy_port = dummy_port.lock();
    *dummy_port.shape().position() = *port.shape().position_ref();
    *dummy_port.anchor() = *port.anchor_ref();
}
