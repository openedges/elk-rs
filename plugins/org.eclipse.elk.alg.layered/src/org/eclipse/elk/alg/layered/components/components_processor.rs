use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkRectangle;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::{
    PortSide, SIDES_EAST, SIDES_EAST_SOUTH, SIDES_EAST_SOUTH_WEST, SIDES_EAST_WEST, SIDES_NONE,
    SIDES_NORTH, SIDES_NORTH_EAST, SIDES_NORTH_EAST_SOUTH, SIDES_NORTH_EAST_SOUTH_WEST,
    SIDES_NORTH_EAST_WEST, SIDES_NORTH_SOUTH, SIDES_NORTH_SOUTH_WEST, SIDES_NORTH_WEST,
    SIDES_SOUTH, SIDES_SOUTH_WEST, SIDES_WEST,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

use crate::org::eclipse::elk::alg::layered::components::{
    ComponentGroup, ComponentOrderingStrategy, ModelOrderComponentGroup,
};
use crate::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LGraphUtil, LNode, LNodeRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions,
};

#[derive(Default)]
pub struct ComponentsProcessor;

impl ComponentsProcessor {
    pub fn new() -> Self {
        ComponentsProcessor
    }

    pub fn split(&self, graph: &LGraphRef) -> Vec<LGraphRef> {
        let (
            separate,
            ext_ports,
            ext_port_constraints,
            consider_model_order,
            nodes,
            graph_props,
            graph_padding,
        ) = if let Ok(mut graph_guard) = graph.lock() {
            (
                graph_guard
                    .get_property(LayeredOptions::SEPARATE_CONNECTED_COMPONENTS)
                    .unwrap_or(true),
                graph_guard
                    .get_property(InternalProperties::GRAPH_PROPERTIES)
                    .unwrap_or_else(EnumSet::none_of)
                    .contains(&GraphProperties::ExternalPorts),
                graph_guard
                    .get_property(LayeredOptions::PORT_CONSTRAINTS)
                    .unwrap_or(PortConstraints::Undefined),
                graph_guard
                    .get_property(LayeredOptions::CONSIDER_MODEL_ORDER_COMPONENTS)
                    .unwrap_or(ComponentOrderingStrategy::None),
                graph_guard.layerless_nodes().clone(),
                graph_guard.graph_element().properties().clone(),
                graph_guard.padding_ref().clone(),
            )
        } else {
            return vec![graph.clone()];
        };

        let compatible_port_constraints = !ext_port_constraints.is_order_fixed();
        if !(separate && (compatible_port_constraints || !ext_ports)) {
            return vec![graph.clone()];
        }

        let mut result: Vec<LGraphRef> = Vec::new();
        let mut visited: HashSet<usize> = HashSet::new();
        for node in &nodes {
            let key = Arc::as_ptr(node) as usize;
            if visited.contains(&key) {
                continue;
            }

            let mut component_nodes: Vec<LNodeRef> = Vec::new();
            let mut ext_port_sides: EnumSet<PortSide> = EnumSet::none_of();
            Self::dfs(
                node,
                &mut visited,
                &mut component_nodes,
                &mut ext_port_sides,
            );

            if component_nodes.is_empty() {
                continue;
            }

            let component_graph = LGraph::new();
            if let Ok(mut component_guard) = component_graph.lock() {
                *component_guard.graph_element().properties_mut() = graph_props.clone();
                component_guard.set_property(
                    InternalProperties::EXT_PORT_CONNECTIONS,
                    Some(ext_port_sides),
                );
                *component_guard.padding() = graph_padding.clone();
                component_guard.set_property(
                    LayeredOptions::NODE_SIZE_MINIMUM,
                    None::<org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector>,
                );

                for component_node in &component_nodes {
                    component_guard
                        .layerless_nodes_mut()
                        .push(component_node.clone());
                    if let Ok(mut node_guard) = component_node.lock() {
                        node_guard.set_graph(&component_graph);
                    }
                }
            }

            result.push(component_graph);
        }

        if consider_model_order != ComponentOrderingStrategy::None {
            result.sort_by_key(LGraphUtil::get_minimal_model_order);
        }

        result
    }

    pub fn combine(&self, components: &[LGraphRef], target: &LGraphRef) {
        if components.len() == 1 {
            let source = components.first().cloned();
            if let Some(source) = source {
                if Arc::ptr_eq(&source, target) {
                    return;
                }

                if let Ok(mut target_guard) = target.lock() {
                    target_guard.layerless_nodes_mut().clear();
                }
                move_graph(target, &source, 0.0, 0.0);

                if let (Ok(mut target_guard), Ok(mut source_guard)) = (target.lock(), source.lock())
                {
                    target_guard
                        .graph_element()
                        .properties_mut()
                        .copy_properties(source_guard.graph_element().properties());
                    *target_guard.padding() = source_guard.padding_ref().clone();
                    target_guard.size().x = source_guard.size_ref().x;
                    target_guard.size().y = source_guard.size_ref().y;
                }
            }
            return;
        }

        if components.is_empty() {
            if let Ok(mut target_guard) = target.lock() {
                target_guard.layerless_nodes_mut().clear();
                target_guard.size().x = 0.0;
                target_guard.size().y = 0.0;
            }
            return;
        }

        let (consider_model_order, has_external_ports) = if let Ok(mut target_guard) = target.lock()
        {
            (
                target_guard
                    .get_property(LayeredOptions::CONSIDER_MODEL_ORDER_COMPONENTS)
                    .unwrap_or(ComponentOrderingStrategy::None),
                target_guard
                    .get_property(InternalProperties::GRAPH_PROPERTIES)
                    .unwrap_or_else(EnumSet::none_of)
                    .contains(&GraphProperties::ExternalPorts),
            )
        } else {
            (ComponentOrderingStrategy::None, false)
        };

        if has_external_ports {
            match consider_model_order {
                ComponentOrderingStrategy::ModelOrder => {
                    combine_model_order_row(components, target);
                }
                ComponentOrderingStrategy::GroupModelOrder => {
                    combine_component_group_model_order(components, target);
                }
                ComponentOrderingStrategy::None
                | ComponentOrderingStrategy::InsidePortSideGroups => {
                    combine_component_group(components, target);
                }
            }
            return;
        }

        combine_simple_row(components, target);
    }

    fn dfs(
        node: &LNodeRef,
        visited: &mut HashSet<usize>,
        component_nodes: &mut Vec<LNodeRef>,
        ext_port_sides: &mut EnumSet<PortSide>,
    ) {
        let key = Arc::as_ptr(node) as usize;
        if !visited.insert(key) {
            return;
        }
        component_nodes.push(node.clone());

        let mut connected_nodes: Vec<LNodeRef> = Vec::new();
        if let Ok(mut node_guard) = node.lock() {
            if node_guard.node_type() == NodeType::ExternalPort {
                if let Some(side) = node_guard.get_property(InternalProperties::EXT_PORT_SIDE) {
                    ext_port_sides.insert(side);
                }
            }

            for port in node_guard.ports().clone() {
                if let Ok(port_guard) = port.lock() {
                    for connected_port in port_guard.connected_ports() {
                        if let Some(connected_node) =
                            connected_port.lock().ok().and_then(|port| port.node())
                        {
                            connected_nodes.push(connected_node);
                        }
                    }
                }
            }
        }

        for connected_node in connected_nodes {
            Self::dfs(&connected_node, visited, component_nodes, ext_port_sides);
        }
    }
}

trait ComponentGroupLike {
    fn get_components_for(&self, connections: &EnumSet<PortSide>) -> Vec<LGraphRef>;
}

impl ComponentGroupLike for ComponentGroup {
    fn get_components_for(&self, connections: &EnumSet<PortSide>) -> Vec<LGraphRef> {
        self.get_components_for(connections)
    }
}

impl ComponentGroupLike for ModelOrderComponentGroup {
    fn get_components_for(&self, connections: &EnumSet<PortSide>) -> Vec<LGraphRef> {
        self.get_components_for(connections)
    }
}

fn combine_simple_row(components: &[LGraphRef], target: &LGraphRef) {
    // Java parity: single-component optimization
    // When there's only one component and it IS the target graph, do nothing (no-op).
    // This prevents double-application of the graph offset.
    if components.len() == 1 {
        if Arc::ptr_eq(&components[0], target) {
            return;
        }
        // Single component but different graph: move and copy properties
        if let Ok(mut target_guard) = target.lock() {
            target_guard.layerless_nodes_mut().clear();
        }
        move_graph(target, &components[0], 0.0, 0.0);
        if let (Ok(mut target_guard), Ok(mut source_guard)) = (target.lock(), components[0].lock()) {
            target_guard
                .graph_element()
                .properties_mut()
                .copy_properties(source_guard.graph_element().properties());
            *target_guard.padding() = source_guard.padding_ref().clone();
            target_guard.size().x = source_guard.size_ref().x;
            target_guard.size().y = source_guard.size_ref().y;
        }
        return;
    }

    let mut ordered_components = components.to_vec();
    sort_components_by_priority(&mut ordered_components, target);

    if let Ok(mut target_guard) = target.lock() {
        target_guard.layerless_nodes_mut().clear();
    }

    if let Some(first_component) = ordered_components.first() {
        if let (Ok(mut target_guard), Ok(mut first_guard)) = (target.lock(), first_component.lock())
        {
            target_guard
                .graph_element()
                .properties_mut()
                .copy_properties(first_guard.graph_element().properties());
        }
    }

    let (aspect_ratio, component_spacing) = if let Ok(mut target_guard) = target.lock() {
        (
            target_guard
                .get_property(LayeredOptions::ASPECT_RATIO)
                .unwrap_or(1.6),
            target_guard
                .get_property(LayeredOptions::SPACING_COMPONENT_COMPONENT)
                .unwrap_or(20.0),
        )
    } else {
        (1.6, 20.0)
    };

    let mut max_row_width = 0.0f64;
    let mut total_area = 0.0f64;
    for component in &ordered_components {
        if let Ok(component_guard) = component.lock() {
            let size = component_guard.size_ref();
            max_row_width = max_row_width.max(size.x);
            total_area += size.x * size.y;
        }
    }
    max_row_width = max_row_width.max(total_area.sqrt() * aspect_ratio);

    place_components_in_rows(
        &ordered_components,
        target,
        max_row_width,
        component_spacing,
    );

    maybe_compact_components(&ordered_components, target, false);

    for component in &ordered_components {
        move_graph(target, component, 0.0, 0.0);
    }
}

fn combine_component_group(components: &[LGraphRef], target: &LGraphRef) {
    if let Ok(mut target_guard) = target.lock() {
        target_guard.layerless_nodes_mut().clear();
    }

    if components.is_empty() {
        if let Ok(mut target_guard) = target.lock() {
            target_guard.size().x = 0.0;
            target_guard.size().y = 0.0;
        }
        return;
    }

    if let Some(first_component) = components.first() {
        if let (Ok(mut target_guard), Ok(mut first_guard)) = (target.lock(), first_component.lock())
        {
            target_guard
                .graph_element()
                .properties_mut()
                .copy_properties(first_guard.graph_element().properties());
        }
    }

    let mut component_groups: Vec<ComponentGroup> = Vec::new();
    for component in components {
        add_component_to_groups(&mut component_groups, component.clone());
    }

    let component_spacing = components
        .first()
        .and_then(|component| component.lock().ok())
        .and_then(|mut component_guard| {
            component_guard.get_property(LayeredOptions::SPACING_COMPONENT_COMPONENT)
        })
        .unwrap_or(20.0);

    let mut offset = KVector::new();
    for group in &component_groups {
        let group_size = place_component_group(group, component_spacing);
        offset_graphs(&group.get_components(), offset.x, offset.y);
        offset.x += group_size.x;
        offset.y += group_size.y;
    }

    if let Ok(mut target_guard) = target.lock() {
        target_guard.size().x = (offset.x - component_spacing).max(0.0);
        target_guard.size().y = (offset.y - component_spacing).max(0.0);
    }

    maybe_compact_components(components, target, true);

    for group in component_groups {
        move_graphs(target, &group.get_components(), 0.0, 0.0);
    }
}

fn combine_component_group_model_order(components: &[LGraphRef], target: &LGraphRef) {
    if let Ok(mut target_guard) = target.lock() {
        target_guard.layerless_nodes_mut().clear();
    }

    if components.is_empty() {
        if let Ok(mut target_guard) = target.lock() {
            target_guard.size().x = 0.0;
            target_guard.size().y = 0.0;
        }
        return;
    }

    if let Some(first_component) = components.first() {
        if let (Ok(mut target_guard), Ok(mut first_guard)) = (target.lock(), first_component.lock())
        {
            target_guard
                .graph_element()
                .properties_mut()
                .copy_properties(first_guard.graph_element().properties());
        }
    }

    let mut component_groups: Vec<ModelOrderComponentGroup> = Vec::new();
    for component in components {
        add_component_to_model_order_groups(&mut component_groups, component.clone());
    }

    let component_spacing = components
        .first()
        .and_then(|component| component.lock().ok())
        .and_then(|mut component_guard| {
            component_guard.get_property(LayeredOptions::SPACING_COMPONENT_COMPONENT)
        })
        .unwrap_or(20.0);

    let direction = target
        .lock()
        .ok()
        .and_then(|mut target_guard| target_guard.get_property(CoreOptions::DIRECTION))
        .unwrap_or(Direction::Right);

    let mut space_blocked_by_south_edges = KVector::new();
    let mut space_blocked_by_components = KVector::new();
    let mut offset = KVector::new();
    let mut max_size = KVector::new();

    for group in &component_groups {
        if direction.is_horizontal() {
            offset.x = space_blocked_by_south_edges.x;
            for side in group.get_port_sides() {
                if side.contains(&PortSide::North) {
                    offset.x = space_blocked_by_components.x;
                    break;
                }
            }
        } else if direction.is_vertical() {
            offset.y = space_blocked_by_south_edges.y;
            for side in group.get_port_sides() {
                if side.contains(&PortSide::West) {
                    offset.y = space_blocked_by_components.y;
                    break;
                }
            }
        }

        let group_size = place_component_group(group, component_spacing);
        offset_graphs(&group.get_components(), offset.x, offset.y);

        if direction.is_horizontal() {
            space_blocked_by_components.x = offset.x + group_size.x;
            max_size.x = max_size.x.max(space_blocked_by_components.x);
            for side in group.get_port_sides() {
                if side.contains(&PortSide::South) {
                    space_blocked_by_south_edges.x = offset.x + group_size.x;
                    break;
                }
            }
            space_blocked_by_components.y = offset.y + group_size.y;
            offset.y = space_blocked_by_components.y;
            max_size.y = max_size.y.max(offset.y);
        } else if direction.is_vertical() {
            space_blocked_by_components.y = offset.y + group_size.y;
            max_size.y = max_size.y.max(space_blocked_by_components.y);
            for side in group.get_port_sides() {
                if side.contains(&PortSide::East) {
                    space_blocked_by_south_edges.y = offset.y + group_size.y;
                    break;
                }
            }
            space_blocked_by_components.x = offset.x + group_size.x;
            offset.x = space_blocked_by_components.x;
            max_size.x = max_size.x.max(offset.x);
        }
    }

    if let Ok(mut target_guard) = target.lock() {
        target_guard.size().x = (max_size.x - component_spacing).max(0.0);
        target_guard.size().y = (max_size.y - component_spacing).max(0.0);
    }

    maybe_compact_components(components, target, true);

    for group in component_groups {
        move_graphs(target, &group.get_components(), 0.0, 0.0);
    }
}

fn add_component_to_groups(component_groups: &mut Vec<ComponentGroup>, component: LGraphRef) {
    for group in component_groups.iter_mut() {
        if group.add(component.clone()) {
            return;
        }
    }

    component_groups.push(ComponentGroup::with_component(component));
}

fn add_component_to_model_order_groups(
    component_groups: &mut Vec<ModelOrderComponentGroup>,
    component: LGraphRef,
) {
    if let Some(group) = component_groups.last_mut() {
        if group.add(component.clone()) {
            return;
        }
    }

    component_groups.push(ModelOrderComponentGroup::with_component(component));
}

fn place_component_group<G: ComponentGroupLike>(group: &G, spacing: f64) -> KVector {
    let size_c = place_components_in_rows_group(&group.get_components_for(&SIDES_NONE), spacing);
    let size_n = place_components_horizontally(&group.get_components_for(&SIDES_NORTH), spacing);
    let size_s = place_components_horizontally(&group.get_components_for(&SIDES_SOUTH), spacing);
    let size_w = place_components_vertically(&group.get_components_for(&SIDES_WEST), spacing);
    let size_e = place_components_vertically(&group.get_components_for(&SIDES_EAST), spacing);
    let size_nw =
        place_components_horizontally(&group.get_components_for(&SIDES_NORTH_WEST), spacing);
    let size_ne =
        place_components_horizontally(&group.get_components_for(&SIDES_NORTH_EAST), spacing);
    let size_sw =
        place_components_horizontally(&group.get_components_for(&SIDES_SOUTH_WEST), spacing);
    let size_se =
        place_components_horizontally(&group.get_components_for(&SIDES_EAST_SOUTH), spacing);
    let size_we = place_components_vertically(&group.get_components_for(&SIDES_EAST_WEST), spacing);
    let size_ns =
        place_components_horizontally(&group.get_components_for(&SIDES_NORTH_SOUTH), spacing);
    let size_nwe =
        place_components_horizontally(&group.get_components_for(&SIDES_NORTH_EAST_WEST), spacing);
    let size_swe =
        place_components_horizontally(&group.get_components_for(&SIDES_EAST_SOUTH_WEST), spacing);
    let size_wns =
        place_components_vertically(&group.get_components_for(&SIDES_NORTH_SOUTH_WEST), spacing);
    let size_ens =
        place_components_vertically(&group.get_components_for(&SIDES_NORTH_EAST_SOUTH), spacing);
    let size_nesw = place_components_horizontally(
        &group.get_components_for(&SIDES_NORTH_EAST_SOUTH_WEST),
        spacing,
    );

    let col_left_width = max_of(&[size_nw.x, size_w.x, size_sw.x, size_wns.x]);
    let col_mid_width = max_of(&[size_n.x, size_c.x, size_s.x, size_nesw.x]);
    let col_ns_width = size_ns.x;
    let col_right_width = max_of(&[size_ne.x, size_e.x, size_se.x, size_ens.x]);
    let row_top_height = max_of(&[size_nw.y, size_n.y, size_ne.y, size_nwe.y]);
    let row_mid_height = max_of(&[size_w.y, size_c.y, size_e.y, size_nesw.y]);
    let row_we_height = size_we.y;
    let row_bottom_height = max_of(&[size_sw.y, size_s.y, size_se.y, size_swe.y]);

    offset_graphs(
        &group.get_components_for(&SIDES_NONE),
        col_left_width + col_ns_width,
        row_top_height + row_we_height,
    );
    offset_graphs(
        &group.get_components_for(&SIDES_NORTH_EAST_SOUTH_WEST),
        col_left_width + col_ns_width,
        row_top_height + row_we_height,
    );
    offset_graphs(
        &group.get_components_for(&SIDES_NORTH),
        col_left_width + col_ns_width,
        0.0,
    );
    offset_graphs(
        &group.get_components_for(&SIDES_SOUTH),
        col_left_width + col_ns_width,
        row_top_height + row_we_height + row_mid_height,
    );
    offset_graphs(
        &group.get_components_for(&SIDES_WEST),
        0.0,
        row_top_height + row_we_height,
    );
    offset_graphs(
        &group.get_components_for(&SIDES_EAST),
        col_left_width + col_ns_width + col_mid_width,
        row_top_height + row_we_height,
    );
    offset_graphs(
        &group.get_components_for(&SIDES_NORTH_EAST),
        col_left_width + col_ns_width + col_mid_width,
        0.0,
    );
    offset_graphs(
        &group.get_components_for(&SIDES_SOUTH_WEST),
        0.0,
        row_top_height + row_we_height + row_mid_height,
    );
    offset_graphs(
        &group.get_components_for(&SIDES_EAST_SOUTH),
        col_left_width + col_ns_width + col_mid_width,
        row_top_height + row_we_height + row_mid_height,
    );
    offset_graphs(
        &group.get_components_for(&SIDES_EAST_WEST),
        0.0,
        row_top_height,
    );
    offset_graphs(
        &group.get_components_for(&SIDES_EAST_SOUTH_WEST),
        0.0,
        row_top_height + row_we_height + row_mid_height,
    );
    offset_graphs(
        &group.get_components_for(&SIDES_NORTH_SOUTH),
        col_left_width,
        0.0,
    );
    offset_graphs(
        &group.get_components_for(&SIDES_NORTH_EAST_SOUTH),
        col_left_width + col_ns_width + col_mid_width,
        0.0,
    );

    KVector::with_values(
        max_of(&[
            col_left_width + col_mid_width + col_ns_width + col_right_width,
            size_we.x,
            size_nwe.x,
            size_swe.x,
        ]),
        max_of(&[
            row_top_height + row_mid_height + row_we_height + row_bottom_height,
            size_ns.y,
            size_wns.y,
            size_ens.y,
        ]),
    )
}

fn place_components_horizontally(components: &[LGraphRef], spacing: f64) -> KVector {
    let mut size = KVector::new();

    for component in components {
        offset_graph(component, size.x, 0.0);
        if let Ok(component_guard) = component.lock() {
            let component_size = component_guard.size_ref();
            size.x += component_size.x + spacing;
            size.y = size.y.max(component_size.y);
        }
    }

    if size.y > 0.0 {
        size.y += spacing;
    }

    size
}

fn place_components_vertically(components: &[LGraphRef], spacing: f64) -> KVector {
    let mut size = KVector::new();

    for component in components {
        offset_graph(component, 0.0, size.y);
        if let Ok(component_guard) = component.lock() {
            let component_size = component_guard.size_ref();
            size.y += component_size.y + spacing;
            size.x = size.x.max(component_size.x);
        }
    }

    if size.x > 0.0 {
        size.x += spacing;
    }

    size
}

fn place_components_in_rows_group(components: &[LGraphRef], spacing: f64) -> KVector {
    if components.is_empty() {
        return KVector::new();
    }

    let mut max_row_width = 0.0f64;
    let mut total_area = 0.0f64;
    for component in components {
        if let Ok(component_guard) = component.lock() {
            let size = component_guard.size_ref();
            max_row_width = max_row_width.max(size.x);
            total_area += size.x * size.y;
        }
    }

    let aspect_ratio = components
        .first()
        .and_then(|component| component.lock().ok())
        .and_then(|mut component_guard| component_guard.get_property(LayeredOptions::ASPECT_RATIO))
        .unwrap_or(1.6);
    max_row_width = max_row_width.max(total_area.sqrt() * aspect_ratio);

    let mut xpos = 0.0f64;
    let mut ypos = 0.0f64;
    let mut highest_box = 0.0f64;
    let mut broadest_row = spacing;

    for component in components {
        let (size_x, size_y) = if let Ok(component_guard) = component.lock() {
            (component_guard.size_ref().x, component_guard.size_ref().y)
        } else {
            (0.0, 0.0)
        };

        if xpos + size_x > max_row_width {
            xpos = 0.0;
            ypos += highest_box + spacing;
            highest_box = 0.0;
        }

        offset_graph(component, xpos, ypos);

        broadest_row = broadest_row.max(xpos + size_x);
        highest_box = highest_box.max(size_y);
        xpos += size_x + spacing;
    }

    KVector::with_values(broadest_row + spacing, ypos + highest_box + spacing)
}

fn combine_model_order_row(components: &[LGraphRef], target: &LGraphRef) {
    if let Ok(mut target_guard) = target.lock() {
        target_guard.layerless_nodes_mut().clear();
    }

    if let Some(first_component) = components.first() {
        if let (Ok(mut target_guard), Ok(mut first_guard)) = (target.lock(), first_component.lock())
        {
            target_guard
                .graph_element()
                .properties_mut()
                .copy_properties(first_guard.graph_element().properties());
        }
    }

    let (aspect_ratio, component_spacing) = if let Ok(mut target_guard) = target.lock() {
        (
            target_guard
                .get_property(LayeredOptions::ASPECT_RATIO)
                .unwrap_or(1.6),
            target_guard
                .get_property(LayeredOptions::SPACING_COMPONENT_COMPONENT)
                .unwrap_or(20.0),
        )
    } else {
        (1.6, 20.0)
    };

    let mut max_row_width = 0.0f64;
    let mut total_area = 0.0f64;
    for component in components {
        if let Ok(component_guard) = component.lock() {
            let size = component_guard.size_ref();
            max_row_width = max_row_width.max(size.x);
            total_area += size.x * size.y;
        }
    }
    max_row_width = max_row_width.max(total_area.sqrt() * aspect_ratio);

    place_components_in_rows_model_order(components, target, max_row_width, component_spacing);

    maybe_compact_components(components, target, false);

    for component in components {
        move_graph(target, component, 0.0, 0.0);
    }
}

fn place_components_in_rows_model_order(
    components: &[LGraphRef],
    target: &LGraphRef,
    max_row_width: f64,
    component_spacing: f64,
) {
    let mut xpos = 0.0f64;
    let mut ypos = 0.0f64;
    let mut highest_box = 0.0f64;
    let mut broadest_row = component_spacing;
    let mut last_component: Option<LGraphRef> = None;
    let mut start_x_of_row = 0.0f64;

    for component in components {
        let (size_x, size_y, offset_x, offset_y, ext_ports) =
            if let Ok(mut component_guard) = component.lock() {
                (
                    component_guard.size_ref().x,
                    component_guard.size_ref().y,
                    component_guard.offset_ref().x,
                    component_guard.offset_ref().y,
                    component_guard
                        .get_property(InternalProperties::EXT_PORT_CONNECTIONS)
                        .unwrap_or_else(EnumSet::none_of),
                )
            } else {
                (0.0, 0.0, 0.0, 0.0, EnumSet::none_of())
            };

        let last_has_east = last_component
            .as_ref()
            .and_then(|last| last.lock().ok())
            .and_then(|mut guard| guard.get_property(InternalProperties::EXT_PORT_CONNECTIONS))
            .is_some_and(|ports| ports.contains(&PortSide::East));

        if (xpos + size_x > max_row_width && !ext_ports.contains(&PortSide::North))
            || last_has_east
            || ext_ports.contains(&PortSide::West)
        {
            xpos = start_x_of_row;
            ypos += highest_box + component_spacing;
            highest_box = 0.0;
        }

        if ext_ports.contains(&PortSide::North) {
            xpos = broadest_row + component_spacing;
        }

        offset_graph(component, xpos + offset_x, ypos + offset_y);
        if let Ok(mut component_guard) = component.lock() {
            component_guard.offset().x = 0.0;
            component_guard.offset().y = 0.0;
        }

        broadest_row = broadest_row.max(xpos + size_x);
        if ext_ports.contains(&PortSide::South) {
            start_x_of_row = start_x_of_row.max(xpos + size_x + component_spacing);
        }
        highest_box = highest_box.max(size_y);
        xpos += size_x + component_spacing;
        last_component = Some(component.clone());
    }

    if let Ok(mut target_guard) = target.lock() {
        target_guard.size().x = broadest_row;
        target_guard.size().y = ypos + highest_box;
    }
}

struct ComponentCompactionEntry {
    graph: LGraphRef,
    bounds: ElkRectangle,
    has_external_connections: bool,
}

fn maybe_compact_components(components: &[LGraphRef], target: &LGraphRef, require_orthogonal: bool) {
    if components.len() < 2 {
        return;
    }

    let (compact_enabled, edge_routing) = components
        .first()
        .and_then(|component| component.lock().ok())
        .map(|mut guard| {
            (
                guard
                    .get_property(LayeredOptions::COMPACTION_CONNECTED_COMPONENTS)
                    .unwrap_or(false),
                guard
                    .get_property(LayeredOptions::EDGE_ROUTING)
                    .unwrap_or(EdgeRouting::Orthogonal),
            )
        })
        .unwrap_or((false, EdgeRouting::Orthogonal));

    if !compact_enabled {
        return;
    }
    if require_orthogonal && edge_routing != EdgeRouting::Orthogonal {
        return;
    }

    // Java parity: component compaction works in a shared absolute coordinate system.
    // Apply each graph's accumulated offset to its nodes first, then reset graph offsets.
    for component in components {
        let (offset_x, offset_y) = component
            .lock()
            .ok()
            .map(|guard| (guard.offset_ref().x, guard.offset_ref().y))
            .unwrap_or((0.0, 0.0));
        if offset_x != 0.0 || offset_y != 0.0 {
            offset_graph(component, offset_x, offset_y);
        }
        if let Ok(mut guard) = component.lock() {
            guard.offset().x = 0.0;
            guard.offset().y = 0.0;
        }
    }

    let mut entries: Vec<ComponentCompactionEntry> = Vec::new();
    for component in components {
        let Some(bounds) = compute_component_bounds(component) else {
            continue;
        };
        let has_external_connections = component
            .lock()
            .ok()
            .and_then(|mut guard| guard.get_property(InternalProperties::EXT_PORT_CONNECTIONS))
            .is_some_and(|connections| !connections.is_empty());
        entries.push(ComponentCompactionEntry {
            graph: component.clone(),
            bounds,
            has_external_connections,
        });
    }

    if entries.len() < 2 {
        return;
    }

    let anchor_x = entries
        .iter()
        .filter(|entry| entry.has_external_connections)
        .map(|entry| entry.bounds.x)
        .fold(f64::INFINITY, f64::min);
    let anchor_x = if anchor_x.is_finite() {
        anchor_x
    } else {
        entries
            .iter()
            .map(|entry| entry.bounds.x)
            .fold(f64::INFINITY, f64::min)
    };
    if !anchor_x.is_finite() {
        return;
    }

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for entry in &entries {
        let delta_x = if entry.has_external_connections {
            0.0
        } else {
            anchor_x - entry.bounds.x
        };
        if delta_x != 0.0 {
            offset_graph(&entry.graph, delta_x, 0.0);
        }

        let shifted_x = entry.bounds.x + delta_x;
        min_x = min_x.min(shifted_x);
        min_y = min_y.min(entry.bounds.y);
        max_x = max_x.max(shifted_x + entry.bounds.width);
        max_y = max_y.max(entry.bounds.y + entry.bounds.height);
    }

    if !(min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite()) {
        return;
    }

    let global_offset = KVector::with_values(-min_x, -min_y);
    let graph_size = KVector::with_values((max_x - min_x).max(0.0), (max_y - min_y).max(0.0));

    for component in components {
        if let Ok(mut guard) = component.lock() {
            guard.offset().x = global_offset.x;
            guard.offset().y = global_offset.y;
        }
    }

    if let Ok(mut target_guard) = target.lock() {
        target_guard.size().x = graph_size.x;
        target_guard.size().y = graph_size.y;
    }
}

fn compute_component_bounds(component: &LGraphRef) -> Option<ElkRectangle> {
    let nodes = collect_component_nodes(component);
    if nodes.is_empty() {
        return component
            .lock()
            .ok()
            .map(|guard| ElkRectangle::with_values(0.0, 0.0, guard.size_ref().x, guard.size_ref().y));
    }

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut has_regular_node = false;

    for node in nodes {
        let Ok(mut node_guard) = node.lock() else {
            continue;
        };
        if node_guard.node_type() == NodeType::ExternalPort {
            continue;
        }
        has_regular_node = true;
        let position = *node_guard.shape().position_ref();
        let size = *node_guard.shape().size_ref();
        let margin = node_guard.margin();

        min_x = min_x.min(position.x - margin.left);
        min_y = min_y.min(position.y - margin.top);
        max_x = max_x.max(position.x + size.x + margin.right);
        max_y = max_y.max(position.y + size.y + margin.bottom);
    }

    if !has_regular_node {
        return component
            .lock()
            .ok()
            .map(|guard| ElkRectangle::with_values(0.0, 0.0, guard.size_ref().x, guard.size_ref().y));
    }

    if !(min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite()) {
        return None;
    }

    Some(ElkRectangle::with_values(
        min_x,
        min_y,
        (max_x - min_x).max(0.0),
        (max_y - min_y).max(0.0),
    ))
}

fn max_of(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .fold(0.0, |acc, value| acc.max(value))
}

fn sort_components_by_priority(components: &mut [LGraphRef], target: &LGraphRef) {
    let consider_model_order = target
        .lock()
        .ok()
        .and_then(|mut target_guard| {
            target_guard.get_property(LayeredOptions::CONSIDER_MODEL_ORDER_COMPONENTS)
        })
        .unwrap_or(ComponentOrderingStrategy::None);

    if consider_model_order != ComponentOrderingStrategy::None {
        let mut keyed: Vec<(usize, i32, LGraphRef)> = components
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, component)| {
                let order = LGraphUtil::get_minimal_model_order(&component);
                (index, order, component)
            })
            .collect();
        keyed.sort_by(
            |(left_index, left_order, _), (right_index, right_order, _)| {
                left_order
                    .cmp(right_order)
                    .then_with(|| left_index.cmp(right_index))
            },
        );
        for (slot, (_, _, component)) in components.iter_mut().zip(keyed) {
            *slot = component;
        }
        return;
    }

    let mut keyed: Vec<(usize, i32, f64, LGraphRef)> = components
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, component)| {
            let (priority, area) = component_priority_and_area(&component);
            (index, priority, area, component)
        })
        .collect();
    keyed.sort_by(
        |(left_index, left_priority, left_area, _),
         (right_index, right_priority, right_area, _)| {
            right_priority
                .cmp(left_priority)
                .then_with(|| left_area.partial_cmp(right_area).unwrap_or(Ordering::Equal))
                .then_with(|| left_index.cmp(right_index))
        },
    );
    for (slot, (_, _, _, component)) in components.iter_mut().zip(keyed) {
        *slot = component;
    }
}

fn component_priority_and_area(graph: &LGraphRef) -> (i32, f64) {
    let priority = collect_component_nodes(graph)
        .iter()
        .filter_map(|node| {
            node.lock()
                .ok()
                .and_then(|mut node_guard| node_guard.get_property(CoreOptions::PRIORITY))
        })
        .sum::<i32>();

    if let Ok(graph_guard) = graph.lock() {
        let size = graph_guard.size_ref();
        (priority, size.x * size.y)
    } else {
        (priority, 0.0)
    }
}

fn place_components_in_rows(
    components: &[LGraphRef],
    target: &LGraphRef,
    max_row_width: f64,
    component_spacing: f64,
) {
    let mut xpos = 0.0f64;
    let mut ypos = 0.0f64;
    let mut highest_box = 0.0f64;
    let mut broadest_row = component_spacing;

    for component in components {
        let (size_x, size_y, offset_x, offset_y) = if let Ok(graph_guard) = component.lock() {
            (
                graph_guard.size_ref().x,
                graph_guard.size_ref().y,
                graph_guard.offset_ref().x,
                graph_guard.offset_ref().y,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        if xpos + size_x > max_row_width {
            xpos = 0.0;
            ypos += highest_box + component_spacing;
            highest_box = 0.0;
        }

        offset_graph(component, xpos + offset_x, ypos + offset_y);
        if let Ok(mut graph_guard) = component.lock() {
            graph_guard.offset().x = 0.0;
            graph_guard.offset().y = 0.0;
        }

        broadest_row = broadest_row.max(xpos + size_x);
        highest_box = highest_box.max(size_y);
        xpos += size_x + component_spacing;
    }

    if let Ok(mut target_guard) = target.lock() {
        target_guard.size().x = broadest_row;
        target_guard.size().y = ypos + highest_box;
    }
}

fn move_graphs(destination: &LGraphRef, sources: &[LGraphRef], offset_x: f64, offset_y: f64) {
    for source in sources {
        move_graph(destination, source, offset_x, offset_y);
    }
}

fn offset_graphs(graphs: &[LGraphRef], offset_x: f64, offset_y: f64) {
    for graph in graphs {
        offset_graph(graph, offset_x, offset_y);
    }
}

fn move_graph(destination: &LGraphRef, source: &LGraphRef, offset_x: f64, offset_y: f64) {
    let (graph_offset_x, graph_offset_y) = if let Ok(source_guard) = source.lock() {
        (
            source_guard.offset_ref().x + offset_x,
            source_guard.offset_ref().y + offset_y,
        )
    } else {
        (offset_x, offset_y)
    };
    let source_nodes = collect_component_nodes(source);

    for node in source_nodes {
        shift_node_and_outgoing_edges(&node, graph_offset_x, graph_offset_y);
        LNode::set_layer(&node, None);
        if let Ok(mut destination_guard) = destination.lock() {
            destination_guard.layerless_nodes_mut().push(node.clone());
        }
        if let Ok(mut node_guard) = node.lock() {
            node_guard.set_graph(destination);
        }
    }
}

fn offset_graph(graph: &LGraphRef, offset_x: f64, offset_y: f64) {
    let nodes = collect_component_nodes(graph);

    for node in nodes {
        shift_node_and_outgoing_edges(&node, offset_x, offset_y);
    }
}

fn collect_component_nodes(graph: &LGraphRef) -> Vec<LNodeRef> {
    let (layerless_nodes, layers) = if let Ok(graph_guard) = graph.lock() {
        (
            graph_guard.layerless_nodes().clone(),
            graph_guard.layers().clone(),
        )
    } else {
        return Vec::new();
    };

    let mut seen: HashSet<usize> = HashSet::new();
    let mut nodes: Vec<LNodeRef> = Vec::new();

    for node in layerless_nodes {
        let key = Arc::as_ptr(&node) as usize;
        if seen.insert(key) {
            nodes.push(node);
        }
    }

    for layer in layers {
        if let Ok(layer_guard) = layer.lock() {
            for node in layer_guard.nodes() {
                let key = Arc::as_ptr(node) as usize;
                if seen.insert(key) {
                    nodes.push(node.clone());
                }
            }
        }
    }

    nodes
}

fn shift_node_and_outgoing_edges(node: &LNodeRef, offset_x: f64, offset_y: f64) {
    let ports = if let Ok(mut node_guard) = node.lock() {
        let position = node_guard.shape().position();
        position.x += offset_x;
        position.y += offset_y;
        node_guard.ports().clone()
    } else {
        Vec::new()
    };

    for port in ports {
        let edges = port
            .lock()
            .ok()
            .map(|port_guard| port_guard.outgoing_edges().clone())
            .unwrap_or_default();

        for edge in edges {
            if let Ok(mut edge_guard) = edge.lock() {
                edge_guard.bend_points().offset(offset_x, offset_y);
                if let Some(mut junction_points) =
                    edge_guard.get_property(LayeredOptions::JUNCTION_POINTS)
                {
                    junction_points.offset(offset_x, offset_y);
                    edge_guard.set_property(LayeredOptions::JUNCTION_POINTS, Some(junction_points));
                }

                let labels = edge_guard.labels().clone();
                drop(edge_guard);
                for label in labels {
                    if let Ok(mut label_guard) = label.lock() {
                        let position = label_guard.shape().position();
                        position.x += offset_x;
                        position.y += offset_y;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;

    use crate::org::eclipse::elk::alg::layered::components::components_processor::sort_components_by_priority;
    use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNode};
    use crate::org::eclipse::elk::alg::layered::options::LayeredMetaDataProvider;

    #[test]
    fn sort_components_by_priority_keeps_input_order_for_ties() {
        LayoutMetaDataService::get_instance()
            .register_layout_meta_data_provider(&LayeredMetaDataProvider);
        let target = LGraph::new();
        let first = LGraph::new();
        let second = LGraph::new();

        if let Ok(mut guard) = first.lock() {
            guard.size().x = 10.0;
            guard.size().y = 10.0;
        }
        if let Ok(mut guard) = second.lock() {
            guard.size().x = 10.0;
            guard.size().y = 10.0;
        }

        let first_node = LNode::new(&first);
        let second_node = LNode::new(&second);
        if let Ok(mut guard) = first.lock() {
            guard.layerless_nodes_mut().push(first_node);
        }
        if let Ok(mut guard) = second.lock() {
            guard.layerless_nodes_mut().push(second_node);
        }

        let mut components = vec![first.clone(), second.clone()];
        sort_components_by_priority(&mut components, &target);

        assert!(Arc::ptr_eq(&components[0], &first));
        assert!(Arc::ptr_eq(&components[1], &second));
    }
}
