#![allow(clippy::mutable_key_type)]

use std::sync::{Arc, Mutex};

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::nodespacing::{
    HorizontalLabelAlignment, LabelCell, VerticalLabelAlignment,
};
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::overlaps::{
    OverlapRemovalDirection, RectangleStripOverlapRemover,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkRectangle, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::label_side::LabelSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::GraphElementAdapter;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::transform::l_graph_adapters::LLabelAdapter;
use crate::org::eclipse::elk::alg::layered::graph::transform::LGraphAdapters;
use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LLabelRef, LNodeRef, LPortRef};
use crate::org::eclipse::elk::alg::layered::options::internal_properties::{
    EndLabelCell, EndLabelMap, PortRefKey,
};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions, Origin};

pub struct EndLabelPreprocessor;

impl ILayoutProcessor<LGraph> for EndLabelPreprocessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("End label pre-processing", 1.0);

        let edge_label_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_LABEL)
            .unwrap_or(2.0);
        let label_label_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_LABEL_LABEL)
            .unwrap_or(1.0);
        let layout_direction = layered_graph
            .get_property(LayeredOptions::DIRECTION)
            .unwrap_or(Direction::Right);
        let vertical_layout = layout_direction.is_vertical();

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                process_node(
                    &node,
                    edge_label_spacing,
                    label_label_spacing,
                    vertical_layout,
                );
            }
        }

        monitor.done();
    }
}

fn process_node(
    node: &LNodeRef,
    edge_label_spacing: f64,
    label_label_spacing: f64,
    vertical_layout: bool,
) {
    let ports = node
        .lock()
        .ok()
        .map(|node_guard| node_guard.ports().clone())
        .unwrap_or_default();
    let port_count = ports.len();
    if port_count == 0 {
        return;
    }

    let mut port_label_cells: Vec<Option<EndLabelCell>> = vec![None; port_count];

    for (port_index, port) in ports.iter().enumerate() {
        let labels = gather_labels(port);
        port_label_cells[port_index] =
            create_configured_label_cell(labels, label_label_spacing, vertical_layout);
    }

    place_labels(
        node,
        &ports,
        &mut port_label_cells,
        label_label_spacing,
        edge_label_spacing,
        vertical_layout,
    );

    let mut port_to_label_cell: EndLabelMap = std::collections::HashMap::new();
    for (index, port) in ports.iter().enumerate() {
        if let Some(cell) = &port_label_cells[index] {
            port_to_label_cell.insert(PortRefKey(port.clone()), cell.clone());
        }
    }

    if !port_to_label_cell.is_empty() {
        if let Ok(mut node_guard) = node.lock() {
            node_guard.set_property(InternalProperties::END_LABELS, Some(port_to_label_cell));
        }
        update_node_margins(node, &port_label_cells);
    }
}

fn create_configured_label_cell(
    labels: Option<Vec<LLabelRef>>,
    label_label_spacing: f64,
    vertical_layout: bool,
) -> Option<EndLabelCell> {
    let labels = labels?;
    if labels.is_empty() {
        return None;
    }

    let mut label_cell: LabelCell<LLabelAdapter, LLabelRef> =
        LabelCell::new_with_layout_mode(label_label_spacing, !vertical_layout);

    for label in labels {
        label_cell.add_label(LGraphAdapters::adapt_label(label));
    }

    let min_height = label_cell.minimum_height();
    let min_width = label_cell.minimum_width();
    let cell_rect = label_cell.cell_rectangle();
    cell_rect.height = min_height;
    cell_rect.width = min_width;

    Some(Arc::new(Mutex::new(label_cell)))
}

// Label gathering

const NO_INCIDENT_EDGE_THICKNESS: f64 = -1.0;

pub(crate) fn gather_labels(port: &LPortRef) -> Option<Vec<LLabelRef>> {
    let mut labels: Vec<LLabelRef> = Vec::new();
    let mut max_edge_thickness = gather_labels_from_port(port, &mut labels);

    let dummy_node = port
        .lock()
        .ok()
        .and_then(|mut port_guard| port_guard.get_property(InternalProperties::PORT_DUMMY));
    if let Some(dummy_node) = dummy_node {
        let dummy_ports = dummy_node
            .lock()
            .ok()
            .map(|dummy_guard| dummy_guard.ports().clone())
            .unwrap_or_default();
        for dummy_port in dummy_ports {
            let origin = dummy_port
                .lock()
                .ok()
                .and_then(|mut port_guard| port_guard.get_property(InternalProperties::ORIGIN));
            let matches_origin = matches!(origin, Some(Origin::LPort(origin_port)) if Arc::ptr_eq(&origin_port, port));
            if matches_origin {
                max_edge_thickness = max_edge_thickness.max(gather_labels_from_port(
                    &dummy_port,
                    &mut labels,
                ));
            }
        }
    }

    if !labels.is_empty() {
        if let Ok(mut port_guard) = port.lock() {
            port_guard.set_property(
                InternalProperties::MAX_EDGE_THICKNESS,
                Some(max_edge_thickness),
            );
        }
    }

    if max_edge_thickness != NO_INCIDENT_EDGE_THICKNESS {
        Some(labels)
    } else {
        None
    }
}

fn gather_labels_from_port(port: &LPortRef, target_list: &mut Vec<LLabelRef>) -> f64 {
    let mut max_edge_thickness = NO_INCIDENT_EDGE_THICKNESS;

    let edges = port
        .lock()
        .ok()
        .map(|port_guard| port_guard.connected_edges())
        .unwrap_or_default();

    for edge in edges {
        let (edge_thickness, is_source, labels) = match edge.lock() {
            Ok(mut edge_guard) => {
                let thickness = edge_guard
                    .get_property(CoreOptions::EDGE_THICKNESS)
                    .unwrap_or(0.0);
                let is_source = edge_guard
                    .source()
                    .map(|source| Arc::ptr_eq(&source, port))
                    .unwrap_or(false);
                (thickness, is_source, edge_guard.labels().clone())
            }
            Err(_) => continue,
        };

        max_edge_thickness = max_edge_thickness.max(edge_thickness);

        for label in labels {
            let placement = match label.lock() {
                Ok(mut label_guard) => label_guard
                    .get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
                    .unwrap_or(EdgeLabelPlacement::Center),
                Err(_) => EdgeLabelPlacement::Center,
            };

            let is_end_label = if is_source {
                placement == EdgeLabelPlacement::Tail
            } else {
                placement == EdgeLabelPlacement::Head
            };

            if is_end_label {
                if let Ok(mut label_guard) = label.lock() {
                    if label_guard
                        .get_property(InternalProperties::END_LABEL_EDGE)
                        .is_none()
                    {
                        label_guard.set_property(InternalProperties::END_LABEL_EDGE, Some(edge.clone()));
                    }
                }
                target_list.push(label);
            }
        }
    }

    max_edge_thickness
}

// Label placement

fn place_labels(
    node: &LNodeRef,
    ports: &[LPortRef],
    port_label_cells: &mut [Option<EndLabelCell>],
    label_label_spacing: f64,
    edge_label_spacing: f64,
    vertical_layout: bool,
) {
    for (index, port) in ports.iter().enumerate() {
        if let Some(cell) = &port_label_cells[index] {
            place_labels_for_port(port, cell, edge_label_spacing);
        }
    }

    let overlap_spacing = 2.0 * label_label_spacing;
    if vertical_layout {
        remove_label_overlaps(
            node,
            ports,
            port_label_cells,
            PortSide::East,
            overlap_spacing,
            edge_label_spacing,
        );
        remove_label_overlaps(
            node,
            ports,
            port_label_cells,
            PortSide::West,
            overlap_spacing,
            edge_label_spacing,
        );
    } else {
        remove_label_overlaps(
            node,
            ports,
            port_label_cells,
            PortSide::North,
            overlap_spacing,
            edge_label_spacing,
        );
        remove_label_overlaps(
            node,
            ports,
            port_label_cells,
            PortSide::South,
            overlap_spacing,
            edge_label_spacing,
        );
    }
}

fn place_labels_for_port(port: &LPortRef, label_cell: &EndLabelCell, edge_label_spacing: f64) {
    let (node_size, node_margin, port_pos, port_anchor, port_side, max_edge_thickness) = {
        let mut port_guard = match port.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let node = match port_guard.node() {
            Some(node) => node,
            None => return,
        };
        let mut node_guard = match node.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        (
            *node_guard.shape().size_ref(),
            node_guard.margin().clone(),
            *port_guard.shape().position_ref(),
            *port_guard.anchor_ref(),
            port_guard.side(),
            port_guard
                .get_property(InternalProperties::MAX_EDGE_THICKNESS)
                .unwrap_or(0.0),
        )
    };

    let port_anchor = KVector::with_values(port_pos.x + port_anchor.x, port_pos.y + port_anchor.y);

    let mut cell_guard = match label_cell.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    let label_side = get_label_side(&cell_guard);
    let (rect_width, rect_height) = {
        let rect = cell_guard.cell_rectangle_ref();
        (rect.width, rect.height)
    };

    match port_side {
        PortSide::North => {
            cell_guard.set_vertical_alignment(VerticalLabelAlignment::Bottom);
            if label_side == LabelSide::Above {
                cell_guard.set_horizontal_alignment(HorizontalLabelAlignment::Right);
                let cell_rect = cell_guard.cell_rectangle();
                cell_rect.y = -node_margin.top - edge_label_spacing - rect_height;
                cell_rect.x = port_anchor.x - max_edge_thickness - edge_label_spacing - rect_width;
            } else {
                cell_guard.set_horizontal_alignment(HorizontalLabelAlignment::Left);
                let cell_rect = cell_guard.cell_rectangle();
                cell_rect.y = -node_margin.top - edge_label_spacing - rect_height;
                cell_rect.x = port_anchor.x + max_edge_thickness + edge_label_spacing;
            }
        }
        PortSide::East => {
            cell_guard.set_horizontal_alignment(HorizontalLabelAlignment::Left);
            if label_side == LabelSide::Above {
                cell_guard.set_vertical_alignment(VerticalLabelAlignment::Bottom);
                let cell_rect = cell_guard.cell_rectangle();
                cell_rect.x = node_size.x + node_margin.right + edge_label_spacing;
                cell_rect.y = port_anchor.y - max_edge_thickness - edge_label_spacing - rect_height;
            } else {
                cell_guard.set_vertical_alignment(VerticalLabelAlignment::Top);
                let cell_rect = cell_guard.cell_rectangle();
                cell_rect.x = node_size.x + node_margin.right + edge_label_spacing;
                cell_rect.y = port_anchor.y + max_edge_thickness + edge_label_spacing;
            }
        }
        PortSide::South => {
            cell_guard.set_vertical_alignment(VerticalLabelAlignment::Top);
            if label_side == LabelSide::Above {
                cell_guard.set_horizontal_alignment(HorizontalLabelAlignment::Right);
                let cell_rect = cell_guard.cell_rectangle();
                cell_rect.y = node_size.y + node_margin.bottom + edge_label_spacing;
                cell_rect.x = port_anchor.x - max_edge_thickness - edge_label_spacing - rect_width;
            } else {
                cell_guard.set_horizontal_alignment(HorizontalLabelAlignment::Left);
                let cell_rect = cell_guard.cell_rectangle();
                cell_rect.y = node_size.y + node_margin.bottom + edge_label_spacing;
                cell_rect.x = port_anchor.x + max_edge_thickness + edge_label_spacing;
            }
        }
        PortSide::West => {
            cell_guard.set_horizontal_alignment(HorizontalLabelAlignment::Right);
            if label_side == LabelSide::Above {
                cell_guard.set_vertical_alignment(VerticalLabelAlignment::Bottom);
                let cell_rect = cell_guard.cell_rectangle();
                cell_rect.x = -node_margin.left - edge_label_spacing - rect_width;
                cell_rect.y = port_anchor.y - max_edge_thickness - edge_label_spacing - rect_height;
            } else {
                cell_guard.set_vertical_alignment(VerticalLabelAlignment::Top);
                let cell_rect = cell_guard.cell_rectangle();
                cell_rect.x = -node_margin.left - edge_label_spacing - rect_width;
                cell_rect.y = port_anchor.y + max_edge_thickness + edge_label_spacing;
            }
        }
        _ => {}
    }
}

fn remove_label_overlaps(
    node: &LNodeRef,
    ports: &[LPortRef],
    port_label_cells: &[Option<EndLabelCell>],
    port_side: PortSide,
    label_label_spacing: f64,
    edge_label_spacing: f64,
) {
    let mut overlap_remover = RectangleStripOverlapRemover::create_for_direction(
        port_side_to_overlap_removal_direction(port_side),
    )
    .with_gap(label_label_spacing, label_label_spacing)
    .with_start_coordinate(calculate_overlap_start_coordinate(
        node,
        port_side,
        edge_label_spacing,
    ));

    let mut label_guards: Vec<std::sync::MutexGuard<'_, LabelCell<LLabelAdapter, LLabelRef>>> =
        Vec::new();

    for (index, port) in ports.iter().enumerate() {
        let matches_side = port
            .lock()
            .ok()
            .map(|port_guard| port_guard.side() == port_side)
            .unwrap_or(false);
        if !matches_side {
            continue;
        }
        let Some(cell) = &port_label_cells[index] else {
            continue;
        };
        if let Ok(mut cell_guard) = cell.lock() {
            let label_cell_rect = cell_guard.cell_rectangle();
            overlap_remover.add_rectangle(label_cell_rect);
            label_guards.push(cell_guard);
        }
    }

    overlap_remover.remove_overlaps();
}

fn calculate_overlap_start_coordinate(
    node: &LNodeRef,
    port_side: PortSide,
    edge_label_spacing: f64,
) -> f64 {
    let (node_size, node_margin) = match node.lock() {
        Ok(mut guard) => (*guard.shape().size_ref(), guard.margin().clone()),
        Err(_) => (KVector::new(), Default::default()),
    };

    match port_side {
        PortSide::North => -node_margin.top - edge_label_spacing,
        PortSide::South => node_size.y + node_margin.bottom + edge_label_spacing,
        PortSide::East => node_size.x + node_margin.right + edge_label_spacing,
        PortSide::West => -node_margin.left - edge_label_spacing,
        _ => 0.0,
    }
}

// Node margins

fn update_node_margins(node: &LNodeRef, label_cells: &[Option<EndLabelCell>]) {
    let (node_size, node_margin) = match node.lock() {
        Ok(mut guard) => (*guard.shape().size_ref(), guard.margin().clone()),
        Err(_) => return,
    };

    let mut node_margin_rect = ElkRectangle::with_values(
        -node_margin.left,
        -node_margin.top,
        node_margin.left + node_size.x + node_margin.right,
        node_margin.top + node_size.y + node_margin.bottom,
    );

    for cell in label_cells.iter().flatten() {
        if let Ok(cell_guard) = cell.lock() {
            node_margin_rect.union(cell_guard.cell_rectangle_ref());
        }
    }

    if let Ok(mut node_guard) = node.lock() {
        let margin = node_guard.margin();
        margin.left = -node_margin_rect.x;
        margin.top = -node_margin_rect.y;
        margin.right = node_margin_rect.width - margin.left - node_size.x;
        margin.bottom = node_margin_rect.height - margin.top - node_size.y;
    }
}

// Utility

fn get_label_side(label_cell: &LabelCell<LLabelAdapter, LLabelRef>) -> LabelSide {
    let first_label = label_cell.labels().first();
    first_label
        .and_then(|label| label.get_property(InternalProperties::LABEL_SIDE))
        .unwrap_or(LabelSide::Unknown)
}

fn port_side_to_overlap_removal_direction(port_side: PortSide) -> OverlapRemovalDirection {
    match port_side {
        PortSide::North => OverlapRemovalDirection::Up,
        PortSide::South => OverlapRemovalDirection::Down,
        PortSide::East => OverlapRemovalDirection::Right,
        PortSide::West => OverlapRemovalDirection::Left,
        _ => OverlapRemovalDirection::Down,
    }
}
