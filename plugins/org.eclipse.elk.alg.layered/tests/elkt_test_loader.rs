#![allow(dead_code)]

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::node_placement_strategy::NodePlacementStrategy;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::NodeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::PortLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkBendPoint, ElkConnectableShapeRef, ElkEdgeRef, ElkEdgeSection, ElkEdgeSectionRef, ElkGraphElementRef,
    ElkLabelRef, ElkNodeRef, ElkPortRef,
};

const DEFAULT_NODE_WIDTH: f64 = 30.0;
const DEFAULT_NODE_HEIGHT: f64 = 30.0;
const DEFAULT_PORT_WIDTH: f64 = 10.0;
const DEFAULT_PORT_HEIGHT: f64 = 10.0;

#[derive(Clone)]
struct NodeDeclaration {
    id: String,
    parent_id: Option<String>,
    has_block: bool,
}

#[derive(Clone)]
struct PortDeclaration {
    id: String,
    node_id: Option<String>,
    has_block: bool,
}

struct EdgeDeclaration {
    id: Option<String>,
    source_ids: Vec<String>,
    target_ids: Vec<String>,
    has_block: bool,
}

enum BlockContext {
    Node(String),
    Port(String),
    Edge(String),
    Label(ElkLabelRef),
}

type LabelDeclaration = (String, String, Option<(f64, f64)>);
type EdgePointDeclaration = (
    String,
    Option<(f64, f64)>,
    Option<(f64, f64)>,
    Option<Vec<(f64, f64)>>,
);
type EdgeSectionGeometry = (
    Option<(f64, f64)>,
    Option<(f64, f64)>,
    Option<Vec<(f64, f64)>>,
);
type SectionLinkParseResult = (Vec<String>, Vec<(String, Vec<String>)>, bool);

struct GenericLabelDeclaration {
    identifier: Option<String>,
    text: String,
    size: Option<(f64, f64)>,
    edge_label_placement: Option<EdgeLabelPlacement>,
    has_block: bool,
}

struct SectionDeclaration {
    id: String,
    outgoing_section_ids: Vec<String>,
    additional_outgoing_links: Vec<(String, Vec<String>)>,
    link_has_empty_target: bool,
    incoming_shape_id: Option<String>,
    outgoing_shape_id: Option<String>,
    geometry: EdgeSectionGeometry,
}

struct PendingSectionLink {
    edge_id: String,
    source_section_id: String,
    target_section_ids: Vec<String>,
    line_number: usize,
    line_text: String,
}

pub fn load_layered_graph_from_elkt(path: &str) -> Result<ElkNodeRef, String> {
    load_layered_graph_from_elk_text(path)
}

pub fn load_layered_graph_from_elk_text(path: &str) -> Result<ElkNodeRef, String> {
    load_graph_from_elkt(path, Some(LayeredOptions::ALGORITHM_ID))
}

pub fn load_graph_from_elkt(path: &str, default_algorithm: Option<&str>) -> Result<ElkNodeRef, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("failed to read ELKT file {path}: {err}"))?;

    let graph = ElkGraphUtil::create_graph();
    if let Some(algorithm_id) = default_algorithm {
        set_node_property(&graph, CoreOptions::ALGORITHM, algorithm_id.to_string());
    }

    let mut nodes: HashMap<String, ElkNodeRef> = HashMap::new();
    let mut ports: HashMap<String, ElkPortRef> = HashMap::new();
    let mut edges: HashMap<String, ElkEdgeRef> = HashMap::new();
    let mut edge_sections_by_edge: HashMap<String, HashMap<String, ElkEdgeSectionRef>> = HashMap::new();
    let mut pending_section_links: Vec<PendingSectionLink> = Vec::new();
    let mut label_identifiers: HashSet<String> = HashSet::new();
    let mut anonymous_edge_counter: usize = 0;
    let mut block_stack: Vec<BlockContext> = Vec::new();

    for (line_index, raw_line) in content.lines().enumerate() {
        let line_number = line_index + 1;
        let mut line = raw_line.trim();
        if line.is_empty()
            || line.starts_with("//")
            || line.starts_with("/*")
            || line.starts_with('*')
            || line.starts_with("*/")
        {
            continue;
        }

        if let Some((before_comment, _)) = line.split_once("//") {
            line = before_comment.trim();
            if line.is_empty() {
                continue;
            }
        }

        let mut trailing_closes = line.chars().filter(|ch| *ch == '}').count();
        line = line.trim_end_matches('}').trim();
        if line.is_empty() {
            pop_block_contexts(&mut block_stack, trailing_closes);
            continue;
        }

        let line_text = line.to_string();
        let mut handled = false;

        if starts_with_statement(line, "nodeProperty") {
            if let Some((node_id, property_name, property_value)) =
                parse_entity_property(line, "nodeProperty")
            {
                let parent = current_node_context(&block_stack);
                let node = find_node_by_identifier_reference(&nodes, node_id.as_str(), parent)
                    .unwrap_or_else(|| {
                        get_or_create_node(&graph, &mut nodes, node_id.as_str(), parent)
                    });
                apply_node_property(&node, property_name, property_value);
                handled = true;
            } else {
                return Err(line_context_error(
                    line_number,
                    line_text.as_str(),
                    "malformed nodeProperty declaration",
                ));
            }
        }

        if !handled && starts_with_statement(line, "portProperty") {
            if let Some((port_id, property_name, property_value)) =
                parse_entity_property(line, "portProperty")
            {
                let Some(port) = find_port_by_identifier_reference(
                    &ports,
                    port_id.as_str(),
                    current_node_context(&block_stack),
                ) else {
                    return Err(line_context_error(
                        line_number,
                        line_text.as_str(),
                        format!("portProperty references unknown port '{port_id}'"),
                    ));
                };
                apply_port_property(&port, property_name, property_value);
                handled = true;
            } else {
                return Err(line_context_error(
                    line_number,
                    line_text.as_str(),
                    "malformed portProperty declaration",
                ));
            }
        }

        if !handled && starts_with_statement(line, "edgeProperty") {
            if let Some((edge_id, property_name, property_value)) =
                parse_entity_property(line, "edgeProperty")
            {
                let Some(edge) = find_edge_by_identifier_reference(
                    &edges,
                    edge_id.as_str(),
                    current_node_context(&block_stack),
                ) else {
                    return Err(line_context_error(
                        line_number,
                        line_text.as_str(),
                        format!("edgeProperty references unknown edge '{edge_id}'"),
                    ));
                };
                apply_edge_property(&edge, property_name, property_value);
                handled = true;
            } else {
                return Err(line_context_error(
                    line_number,
                    line_text.as_str(),
                    "malformed edgeProperty declaration",
                ));
            }
        }

        if !handled && starts_with_statement(line, "port") {
            if let Some(port_decl) = parse_port_declaration(line) {
                let parent_id = port_decl
                    .node_id
                    .as_deref()
                    .or_else(|| current_node_context(&block_stack));
                let port_scope_key = port_storage_key(port_decl.id.as_str(), parent_id);
                let port = get_or_create_port(
                    &graph,
                    &mut nodes,
                    &mut ports,
                    port_decl.id.as_str(),
                    parent_id,
                );
                let _ = apply_port_block_line(&port, line);
                if let Some(side_value) = parse_inline_value(line, "side") {
                    if let Some(side) = parse_port_side(side_value.as_str()) {
                        set_port_side_property(&port, side);
                    }
                }
                if port_decl.has_block {
                    apply_block_open(
                        &mut block_stack,
                        &mut trailing_closes,
                        BlockContext::Port(port_scope_key),
                    );
                }
                handled = true;
            } else {
                return Err(line_context_error(
                    line_number,
                    line_text.as_str(),
                    "malformed port declaration",
                ));
            }
        }

        if !handled && starts_with_statement(line, "node") {
            if let Some(node_decl) = parse_node_declaration(line) {
                let parent_id = node_decl
                    .parent_id
                    .as_deref()
                    .or_else(|| current_node_context(&block_stack));
                let node_scope_key = node_storage_key(node_decl.id.as_str(), parent_id);
                let node = get_or_create_node(&graph, &mut nodes, node_decl.id.as_str(), parent_id);
                let _ = apply_node_block_line(&node, line);
                if let Some(value) = parse_inline_value(line, "portConstraints") {
                    if let Some(constraints) = parse_port_constraints(value.as_str()) {
                        set_node_property(&node, LayeredOptions::PORT_CONSTRAINTS, constraints);
                    }
                }
                if node_decl.has_block {
                    apply_block_open(
                        &mut block_stack,
                        &mut trailing_closes,
                        BlockContext::Node(node_scope_key),
                    );
                }
                handled = true;
            } else {
                return Err(line_context_error(
                    line_number,
                    line_text.as_str(),
                    "malformed node declaration",
                ));
            }
        }

        if !handled && starts_with_statement(line, "edge") {
            if let Some(edge_decl) = parse_edge_declaration(line) {
                let sources = edge_decl
                    .source_ids
                    .iter()
                    .map(|identifier| {
                        resolve_connectable(
                            &graph,
                            &mut nodes,
                            &mut ports,
                            identifier,
                            current_node_context(&block_stack),
                        )
                    })
                    .collect::<Vec<_>>();
                let targets = edge_decl
                    .target_ids
                    .iter()
                    .map(|identifier| {
                        resolve_connectable(
                            &graph,
                            &mut nodes,
                            &mut ports,
                            identifier,
                            current_node_context(&block_stack),
                        )
                    })
                    .collect::<Vec<_>>();
                let edge = ElkGraphUtil::create_hyperedge(sources, targets);
                let edge_id = if let Some(edge_id) = edge_decl.id {
                    let scoped_edge_id =
                        edge_storage_key(edge_id.as_str(), current_node_context(&block_stack));
                    if edges.contains_key(scoped_edge_id.as_str()) {
                        return Err(line_context_error(
                            line_number,
                            line_text.as_str(),
                            format!("duplicate edge identifier: {edge_id}"),
                        ));
                    }
                    scoped_edge_id
                } else {
                    loop {
                        anonymous_edge_counter += 1;
                        let candidate = format!("__anonymous_edge_{anonymous_edge_counter}");
                        if !edges.contains_key(candidate.as_str()) {
                            break candidate;
                        }
                    }
                };
                edges.insert(edge_id.clone(), edge.clone());
                if edge_decl.has_block {
                    apply_block_open(
                        &mut block_stack,
                        &mut trailing_closes,
                        BlockContext::Edge(edge_id),
                    );
                }
                handled = true;
            } else {
                return Err(line_context_error(
                    line_number,
                    line_text.as_str(),
                    "malformed edge declaration",
                ));
            }
        }

        if !handled && starts_with_statement(line, "edgeSection") {
            if let Some((edge_id, source_point, target_point, bend_points)) =
                parse_edge_section_declaration(line)
            {
                let Some(edge) = find_edge_by_identifier_reference(
                    &edges,
                    edge_id.as_str(),
                    current_node_context(&block_stack),
                ) else {
                    return Err(line_context_error(
                        line_number,
                        line_text.as_str(),
                        format!("edgeSection references unknown edge '{edge_id}'"),
                    ));
                };
                apply_edge_section(&edge, source_point, target_point, bend_points, true);
                handled = true;
            } else {
                return Err(line_context_error(
                    line_number,
                    line_text.as_str(),
                    "malformed edgeSection declaration",
                ));
            }
        }

        if !handled && starts_with_statement(line, "edgePoint") {
            if let Some((edge_id, source_point, target_point, bend_points)) =
                parse_edge_point_declaration(line)
            {
                let Some(edge) = find_edge_by_identifier_reference(
                    &edges,
                    edge_id.as_str(),
                    current_node_context(&block_stack),
                ) else {
                    return Err(line_context_error(
                        line_number,
                        line_text.as_str(),
                        format!("edgePoint references unknown edge '{edge_id}'"),
                    ));
                };
                apply_edge_section(&edge, source_point, target_point, bend_points, false);
                handled = true;
            } else {
                return Err(line_context_error(
                    line_number,
                    line_text.as_str(),
                    "malformed edgePoint declaration",
                ));
            }
        }

        if !handled && starts_with_statement(line, "section") {
            if let Some(section_decl) = parse_section_declaration(line) {
                if section_decl.link_has_empty_target {
                    return Err(line_context_error(
                        line_number,
                        line_text.as_str(),
                        format!(
                            "section link has empty target section list for section '{}'",
                            section_decl.id
                        ),
                    ));
                }
                let Some(edge_id) = current_edge_context(&block_stack) else {
                    return Err(line_context_error(
                        line_number,
                        line_text.as_str(),
                        format!("section declaration must be inside edge block: {line}"),
                    ));
                };
                let Some(edge) = edges.get(edge_id) else {
                    return Err(line_context_error(
                        line_number,
                        line_text.as_str(),
                        format!("section declaration references unknown edge: {edge_id}"),
                    ));
                };

                let section_map = edge_sections_by_edge.entry(edge_id.to_string()).or_default();
                let section =
                    get_or_create_edge_section_for_identifier(edge, section_map, &section_decl.id);
                apply_edge_section_to(
                    &section,
                    section_decl.geometry.0,
                    section_decl.geometry.1,
                    section_decl.geometry.2,
                    true,
                );

                if let Some(outgoing_shape_id) = section_decl.outgoing_shape_id.as_deref() {
                    let outgoing =
                        resolve_existing_connectable(&nodes, &ports, outgoing_shape_id).ok_or_else(
                            || {
                                line_context_error(
                                    line_number,
                                    line_text.as_str(),
                                    format!(
                                        "section '{}' in edge '{}' references unknown outgoing shape '{}'",
                                        section_decl.id, edge_id, outgoing_shape_id
                                    ),
                                )
                            },
                        )?;
                    section.borrow_mut().set_outgoing_shape(Some(outgoing));
                }
                if let Some(incoming_shape_id) = section_decl.incoming_shape_id.as_deref() {
                    let incoming =
                        resolve_existing_connectable(&nodes, &ports, incoming_shape_id).ok_or_else(
                            || {
                                line_context_error(
                                    line_number,
                                    line_text.as_str(),
                                    format!(
                                        "section '{}' in edge '{}' references unknown incoming shape '{}'",
                                        section_decl.id, edge_id, incoming_shape_id
                                    ),
                                )
                            },
                        )?;
                    section.borrow_mut().set_incoming_shape(Some(incoming));
                }
                if !section_decl.outgoing_section_ids.is_empty() {
                    pending_section_links.push(PendingSectionLink {
                        edge_id: edge_id.to_string(),
                        source_section_id: section_decl.id.clone(),
                        target_section_ids: section_decl.outgoing_section_ids,
                        line_number,
                        line_text: line_text.clone(),
                    });
                }
                for (source_id, target_ids) in section_decl.additional_outgoing_links {
                    if !target_ids.is_empty() {
                        pending_section_links.push(PendingSectionLink {
                            edge_id: edge_id.to_string(),
                            source_section_id: source_id,
                            target_section_ids: target_ids,
                            line_number,
                            line_text: line_text.clone(),
                        });
                    }
                }
                handled = true;
            } else {
                return Err(line_context_error(
                    line_number,
                    line_text.as_str(),
                    "malformed section declaration",
                ));
            }
        }

        if !handled && starts_with_statement(line, "edgeLabel") {
            if let Some((edge_id, text, size)) = parse_edge_label_declaration(line) {
                let Some(edge) = find_edge_by_identifier_reference(
                    &edges,
                    edge_id.as_str(),
                    current_node_context(&block_stack),
                ) else {
                    return Err(line_context_error(
                        line_number,
                        line_text.as_str(),
                        format!("edgeLabel references unknown edge '{edge_id}'"),
                    ));
                };
                let label = ElkGraphUtil::create_label_with_text(
                    &text,
                    Some(ElkGraphElementRef::Edge(edge.clone())),
                );
                if let Some((width, height)) = size {
                    label.borrow_mut().shape().set_dimensions(width, height);
                }
                if let Some(value) = parse_inline_field_value(line, "placement")
                    .or_else(|| parse_inline_field_value(line, "edgeLabelPlacement"))
                {
                    if let Some(placement) = parse_edge_label_placement(value.as_str()) {
                        set_label_property(&label, LayeredOptions::EDGE_LABELS_PLACEMENT, placement);
                    }
                }
                handled = true;
            } else {
                return Err(line_context_error(
                    line_number,
                    line_text.as_str(),
                    "malformed edgeLabel declaration",
                ));
            }
        }

        if !handled && starts_with_statement(line, "nodeLabel") {
            if let Some((node_id, text, size)) = parse_node_label_declaration(line) {
                let parent = current_node_context(&block_stack);
                let Some(node) = find_node_by_identifier_reference(&nodes, node_id.as_str(), parent) else {
                    return Err(line_context_error(
                        line_number,
                        line_text.as_str(),
                        format!("nodeLabel references unknown node '{node_id}'"),
                    ));
                };
                let label = ElkGraphUtil::create_label_with_text(
                    &text,
                    Some(ElkGraphElementRef::Node(node.clone())),
                );
                if let Some((width, height)) = size {
                    label.borrow_mut().shape().set_dimensions(width, height);
                }
                handled = true;
            } else {
                return Err(line_context_error(
                    line_number,
                    line_text.as_str(),
                    "malformed nodeLabel declaration",
                ));
            }
        }

        if !handled && starts_with_statement(line, "portLabel") {
            if let Some((port_id, text, size)) = parse_port_label_declaration(line) {
                let Some(port) = find_port_by_identifier_reference(
                    &ports,
                    port_id.as_str(),
                    current_node_context(&block_stack),
                ) else {
                    return Err(line_context_error(
                        line_number,
                        line_text.as_str(),
                        format!("portLabel references unknown port '{port_id}'"),
                    ));
                };
                let label = ElkGraphUtil::create_label_with_text(
                    &text,
                    Some(ElkGraphElementRef::Port(port.clone())),
                );
                if let Some((width, height)) = size {
                    label.borrow_mut().shape().set_dimensions(width, height);
                }
                handled = true;
            } else {
                return Err(line_context_error(
                    line_number,
                    line_text.as_str(),
                    "malformed portLabel declaration",
                ));
            }
        }

        if !handled && starts_with_statement(line, "label") {
            if let Some(label_decl) = parse_label_declaration(line) {
                let (parent, parent_scope) = if let Some(label) = current_label_context(&block_stack) {
                    (
                        Some(ElkGraphElementRef::Label(label.clone())),
                        Some(format!("label:{:p}", std::rc::Rc::as_ptr(&label))),
                    )
                } else if let Some(port_id) = current_port_context(&block_stack) {
                    ports
                        .get(port_id)
                        .map(|port| {
                            (
                                ElkGraphElementRef::Port(port.clone()),
                                format!("port:{:p}", std::rc::Rc::as_ptr(port)),
                            )
                        })
                        .map_or((None, None), |(parent, scope)| (Some(parent), Some(scope)))
                } else if let Some(edge_id) = current_edge_context(&block_stack) {
                    edges
                        .get(edge_id)
                        .map(|edge| {
                            (
                                ElkGraphElementRef::Edge(edge.clone()),
                                format!("edge:{:p}", std::rc::Rc::as_ptr(edge)),
                            )
                        })
                        .map_or((None, None), |(parent, scope)| (Some(parent), Some(scope)))
                } else if let Some(node_id) = current_node_context(&block_stack) {
                    nodes
                        .get(node_id)
                        .map(|node| {
                            (
                                ElkGraphElementRef::Node(node.clone()),
                                format!("node:{:p}", std::rc::Rc::as_ptr(node)),
                            )
                        })
                        .map_or((None, None), |(parent, scope)| (Some(parent), Some(scope)))
                } else {
                    (None, None)
                };

                let Some(parent) = parent else {
                    return Err(line_context_error(
                        line_number,
                        line_text.as_str(),
                        "label declaration must be inside node/port/edge/label block",
                    ));
                };

                let label = ElkGraphUtil::create_label_with_text(&label_decl.text, Some(parent));
                if let Some(identifier) = label_decl.identifier {
                    register_label_identifier(
                        &mut label_identifiers,
                        parent_scope.as_deref().unwrap_or("unknown"),
                        identifier.as_str(),
                    )
                        .map_err(|err| {
                            line_context_error(line_number, line_text.as_str(), err)
                        })?;
                    label
                        .borrow_mut()
                        .shape()
                        .graph_element()
                        .set_identifier(Some(identifier));
                }
                if let Some((width, height)) = label_decl.size {
                    label.borrow_mut().shape().set_dimensions(width, height);
                }
                if let Some(placement) = label_decl.edge_label_placement {
                    set_label_property(&label, LayeredOptions::EDGE_LABELS_PLACEMENT, placement);
                }
                if label_decl.has_block {
                    apply_block_open(
                        &mut block_stack,
                        &mut trailing_closes,
                        BlockContext::Label(label),
                    );
                }
                handled = true;
            } else {
                return Err(line_context_error(
                    line_number,
                    line_text.as_str(),
                    "malformed label declaration",
                ));
            }
        }

        if !handled {
            if let Some(label) = current_label_context(&block_stack) {
                handled = apply_label_block_line(&label, line);
            }
        }

        if !handled {
            if let Some(edge_id) = current_edge_context(&block_stack) {
                if let Some(edge) = edges.get(edge_id) {
                    handled = apply_edge_block_line(edge, line);
                }
            }
        }

        if !handled {
            if let Some(port_id) = current_port_context(&block_stack) {
                if let Some(port) = ports.get(port_id) {
                    handled = apply_port_block_line(port, line);
                }
            }
        }

        if !handled {
            if let Some(node_id) = current_node_context(&block_stack) {
                if let Some(node) = nodes.get(node_id) {
                    handled = apply_node_block_line(node, line);
                }
            }
        }

        if !handled {
            let _ = apply_graph_property_line(&graph, line);
        }

        pop_block_contexts(&mut block_stack, trailing_closes);
    }

    infer_missing_port_sides(&graph);
    resolve_pending_section_links(&edge_sections_by_edge, &pending_section_links)?;

    Ok(graph)
}

fn pop_block_contexts(stack: &mut Vec<BlockContext>, count: usize) {
    for _ in 0..count {
        if stack.pop().is_none() {
            break;
        }
    }
}

fn apply_block_open(stack: &mut Vec<BlockContext>, trailing_closes: &mut usize, context: BlockContext) {
    if *trailing_closes > 0 {
        *trailing_closes -= 1;
    } else {
        stack.push(context);
    }
}

fn current_node_context(stack: &[BlockContext]) -> Option<&str> {
    stack.iter().rev().find_map(|entry| match entry {
        BlockContext::Node(identifier) => Some(identifier.as_str()),
        _ => None,
    })
}

fn current_port_context(stack: &[BlockContext]) -> Option<&str> {
    match stack.last() {
        Some(BlockContext::Port(identifier)) => Some(identifier.as_str()),
        _ => None,
    }
}

fn current_edge_context(stack: &[BlockContext]) -> Option<&str> {
    match stack.last() {
        Some(BlockContext::Edge(identifier)) => Some(identifier.as_str()),
        _ => None,
    }
}

fn current_label_context(stack: &[BlockContext]) -> Option<ElkLabelRef> {
    match stack.last() {
        Some(BlockContext::Label(label)) => Some(label.clone()),
        _ => None,
    }
}

fn starts_with_statement(line: &str, keyword: &str) -> bool {
    line.strip_prefix(keyword)
        .and_then(|rest| rest.chars().next())
        .is_some_and(|ch| ch.is_whitespace())
}

fn apply_graph_property_line(graph: &ElkNodeRef, line: &str) -> bool {
    if let Some(value) = parse_key_value(line, "algorithm") {
        if value.eq_ignore_ascii_case("layered") {
            set_node_property(graph, CoreOptions::ALGORITHM, LayeredOptions::ALGORITHM_ID.to_string());
        }
        return true;
    }

    if let Some(value) = parse_key_value(line, "spacing.nodeNode") {
        if let Ok(spacing) = value.parse::<f64>() {
            set_node_property(graph, CoreOptions::SPACING_NODE_NODE, spacing);
        }
        return true;
    }

    if let Some(value) = parse_key_value(line, "spacing.edgeEdge") {
        if let Ok(spacing) = value.parse::<f64>() {
            set_node_property(graph, CoreOptions::SPACING_EDGE_EDGE, spacing);
        }
        return true;
    }

    if let Some(value) = parse_key_value(line, "direction") {
        if let Some(direction) = parse_direction(value) {
            set_node_property(graph, CoreOptions::DIRECTION, direction);
        }
        return true;
    }

    if let Some(value) = parse_key_value(line, "edgeRouting") {
        if let Some(routing) = parse_edge_routing(value) {
            set_node_property(graph, CoreOptions::EDGE_ROUTING, routing);
        }
        return true;
    }

    if let Some(value) = parse_key_value(line, "hierarchyHandling") {
        if let Some(handling) = parse_hierarchy_handling(value) {
            set_node_property(graph, CoreOptions::HIERARCHY_HANDLING, handling);
        }
        return true;
    }

    if let Some(value) = parse_key_value(line, "insideSelfLoops.activate") {
        if let Some(enabled) = parse_bool(value) {
            set_node_property(graph, CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE, enabled);
        }
        return true;
    }

    if let Some(value) = parse_key_value(line, "padding") {
        if let Some(padding) = parse_padding(value) {
            set_node_property(graph, CoreOptions::PADDING, padding);
        }
        return true;
    }

    if let Some(value) = parse_key_value(line, "nodeLabelsPadding") {
        if let Some(padding) = parse_padding(value) {
            set_node_property(graph, LayeredOptions::NODE_LABELS_PADDING, padding);
        }
        return true;
    }

    if let Some(value) = parse_key_value(line, "nodeLabelsPlacement") {
        if let Some(placement) = parse_node_label_placement(value) {
            set_node_property(graph, LayeredOptions::NODE_LABELS_PLACEMENT, placement);
        }
        return true;
    }

    if let Some(value) = parse_key_value(line, "nodePlacementStrategy") {
        if let Some(strategy) = parse_node_placement_strategy(value) {
            set_node_property(graph, LayeredOptions::NODE_PLACEMENT_STRATEGY, strategy);
        }
        return true;
    }

    if let Some(value) = parse_key_value(line, "portLabelsPlacement") {
        if let Some(placement) = parse_port_label_placement(value) {
            set_node_property(graph, CoreOptions::PORT_LABELS_PLACEMENT, placement);
        }
        return true;
    }

    false
}

fn apply_node_block_line(node: &ElkNodeRef, line: &str) -> bool {
    let mut applied = false;

    if let Some((x, y)) = parse_position(line) {
        node.borrow_mut().connectable().shape().set_location(x, y);
        applied = true;
    }

    if let Some((width, height)) = parse_size(line) {
        set_node_dimensions(node, width, height);
        applied = true;
    }

    if let Some((property_name, property_value)) = parse_property_line(line) {
        apply_node_property(node, property_name, property_value);
        applied = true;
    }

    applied
}

fn apply_port_block_line(port: &ElkPortRef, line: &str) -> bool {
    let mut applied = false;

    if let Some((x, y)) = parse_position(line) {
        port.borrow_mut().connectable().shape().set_location(x, y);
        applied = true;
    }

    if let Some((width, height)) = parse_size(line) {
        set_port_dimensions(port, width, height);
        applied = true;
    }

    if let Some((property_name, property_value)) = parse_property_line(line) {
        apply_port_property(port, property_name, property_value);
        applied = true;
    }

    applied
}

fn apply_label_block_line(label: &ElkLabelRef, line: &str) -> bool {
    let mut applied = false;

    if let Some((x, y)) = parse_position(line) {
        label.borrow_mut().shape().set_location(x, y);
        applied = true;
    }

    if let Some((width, height)) = parse_size(line) {
        label.borrow_mut().shape().set_dimensions(width, height);
        applied = true;
    }

    if let Some((property_name, property_value)) = parse_property_line(line) {
        match normalize_property_key(property_name).as_str() {
            "placement" | "edgelabelplacement" | "edgelabelsplacement" => {
                if let Some(placement) = parse_edge_label_placement(property_value) {
                    set_label_property(label, LayeredOptions::EDGE_LABELS_PLACEMENT, placement);
                    applied = true;
                }
            }
            _ => {}
        }
    }

    applied
}

fn apply_edge_block_line(edge: &ElkEdgeRef, line: &str) -> bool {
    if let Some((source_point, target_point, bend_points)) = parse_edge_layout_line(line) {
        apply_edge_section(edge, source_point, target_point, bend_points, true);
        return true;
    }

    if let Some((property_name, property_value)) = parse_property_line(line) {
        apply_edge_property(edge, property_name, property_value);
        return true;
    }

    false
}

fn parse_edge_layout_line(
    line: &str,
) -> Option<EdgeSectionGeometry> {
    if !line.starts_with("layout") {
        return None;
    }

    let source_point = parse_named_value_any(
        line,
        &["start", "source", "sourcePoint"],
        &["end", "target", "targetPoint", "bends", "bend", "bendPoint", "bendPoints"],
    )
    .and_then(parse_point_pair);
    let target_point = parse_named_value_any(
        line,
        &["end", "target", "targetPoint"],
        &["start", "source", "sourcePoint", "bends", "bend", "bendPoint", "bendPoints"],
    )
    .and_then(parse_point_pair);
    let bend_points = parse_named_value_any(
        line,
        &["bends", "bend", "bendPoint", "bendPoints"],
        &["start", "source", "sourcePoint", "end", "target", "targetPoint"],
    )
    .map(parse_point_pairs);

    if source_point.is_none() && target_point.is_none() && bend_points.is_none() {
        None
    } else {
        Some((source_point, target_point, bend_points))
    }
}

fn resolve_connectable(
    graph: &ElkNodeRef,
    nodes: &mut HashMap<String, ElkNodeRef>,
    ports: &mut HashMap<String, ElkPortRef>,
    identifier: &str,
    current_node_id: Option<&str>,
) -> ElkConnectableShapeRef {
    if let Some(port) = find_port_by_identifier_reference(ports, identifier, current_node_id) {
        return ElkConnectableShapeRef::Port(port.clone());
    }

    if let Some((node_id, port_id)) = identifier.split_once('.') {
        let parent_storage_key = find_node_storage_key_reference(nodes, node_id, current_node_id)
            .unwrap_or_else(|| node_storage_key(node_id, current_node_id));
        let port = get_or_create_port(
            graph,
            nodes,
            ports,
            port_id.trim(),
            Some(parent_storage_key.as_str()),
        );
        return ElkConnectableShapeRef::Port(port);
    }

    if let Some(node) = find_node_by_identifier_reference(nodes, identifier, current_node_id) {
        return ElkConnectableShapeRef::Node(node);
    }

    let node = get_or_create_node(graph, nodes, identifier, current_node_id);
    ElkConnectableShapeRef::Node(node)
}

fn resolve_existing_connectable(
    nodes: &HashMap<String, ElkNodeRef>,
    ports: &HashMap<String, ElkPortRef>,
    identifier: &str,
) -> Option<ElkConnectableShapeRef> {
    if let Some(port) = find_port_by_identifier_reference(ports, identifier, None) {
        return Some(ElkConnectableShapeRef::Port(port.clone()));
    }

    find_node_by_identifier_reference(nodes, identifier, None).map(ElkConnectableShapeRef::Node)
}

fn edge_storage_key(identifier: &str, current_node_id: Option<&str>) -> String {
    let identifier = identifier.trim();
    if let Some(current_node_id) = current_node_id {
        let current_node_id = current_node_id.trim();
        if !current_node_id.is_empty() {
            return format!("{current_node_id}.{identifier}");
        }
    }
    identifier.to_string()
}

fn find_edge_by_identifier_reference(
    edges: &HashMap<String, ElkEdgeRef>,
    identifier: &str,
    current_node_id: Option<&str>,
) -> Option<ElkEdgeRef> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return None;
    }

    if let Some(current_node_id) = current_node_id {
        let scoped_key = edge_storage_key(identifier, Some(current_node_id));
        if let Some(edge) = edges.get(scoped_key.as_str()) {
            return Some(edge.clone());
        }
    }

    if let Some(edge) = edges.get(identifier) {
        return Some(edge.clone());
    }

    let suffix = format!(".{identifier}");
    let mut matches = edges
        .iter()
        .filter(|(key, _)| key.ends_with(suffix.as_str()))
        .map(|(_, edge)| edge.clone());
    let first = matches.next();
    if first.is_some() && matches.next().is_none() {
        return first;
    }

    None
}

fn node_storage_key(identifier: &str, parent_node_id: Option<&str>) -> String {
    let identifier = identifier.trim();
    if let Some(parent_node_id) = parent_node_id {
        let parent_node_id = parent_node_id.trim();
        if !parent_node_id.is_empty() {
            return format!("{parent_node_id}.{identifier}");
        }
    }
    identifier.to_string()
}

fn find_node_by_identifier_reference(
    nodes: &HashMap<String, ElkNodeRef>,
    identifier: &str,
    parent_node_id: Option<&str>,
) -> Option<ElkNodeRef> {
    let storage_key = find_node_storage_key_reference(nodes, identifier, parent_node_id)?;
    nodes.get(storage_key.as_str()).cloned()
}

fn find_node_storage_key_reference(
    nodes: &HashMap<String, ElkNodeRef>,
    identifier: &str,
    parent_node_id: Option<&str>,
) -> Option<String> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return None;
    }

    if let Some(parent_node_id) = parent_node_id {
        let scoped_key = node_storage_key(identifier, Some(parent_node_id));
        if nodes.contains_key(scoped_key.as_str()) {
            return Some(scoped_key);
        }
    }

    if nodes.contains_key(identifier) {
        return Some(identifier.to_string());
    }

    let suffix = format!(".{identifier}");
    let mut matches = nodes
        .iter()
        .filter(|(key, _)| key.ends_with(suffix.as_str()))
        .map(|(key, _)| key.to_string());
    let first = matches.next();
    if first.is_some() && matches.next().is_none() {
        return first;
    }

    None
}

fn port_storage_key(identifier: &str, parent_node_id: Option<&str>) -> String {
    let identifier = identifier.trim();
    if let Some(parent_node_id) = parent_node_id {
        let parent_node_id = parent_node_id.trim();
        if !parent_node_id.is_empty() {
            return format!("{parent_node_id}.{identifier}");
        }
    }
    identifier.to_string()
}

fn find_port_by_identifier_reference(
    ports: &HashMap<String, ElkPortRef>,
    identifier: &str,
    parent_node_id: Option<&str>,
) -> Option<ElkPortRef> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return None;
    }

    if let Some(parent_node_id) = parent_node_id {
        let scoped_key = port_storage_key(identifier, Some(parent_node_id));
        if let Some(port) = ports.get(scoped_key.as_str()) {
            return Some(port.clone());
        }
    }

    if let Some(port) = ports.get(identifier) {
        return Some(port.clone());
    }

    if let Some((node_id, port_id)) = identifier.split_once('.') {
        let scoped_key = port_storage_key(port_id, Some(node_id));
        if let Some(port) = ports.get(scoped_key.as_str()) {
            return Some(port.clone());
        }
    }

    let suffix = format!(".{identifier}");
    let mut matches = ports
        .iter()
        .filter(|(key, _)| key.ends_with(suffix.as_str()))
        .map(|(_, port)| port.clone());
    let first = matches.next();
    if first.is_some() && matches.next().is_none() {
        return first;
    }

    None
}

fn apply_node_property(node: &ElkNodeRef, property_name: &str, property_value: &str) {
    match normalize_property_key(property_name).as_str() {
        "algorithm" => {
            if property_value.eq_ignore_ascii_case("layered") {
                set_node_property(node, CoreOptions::ALGORITHM, LayeredOptions::ALGORITHM_ID.to_string());
            }
        }
        "hierarchyhandling" => {
            if let Some(handling) = parse_hierarchy_handling(property_value) {
                set_node_property(node, CoreOptions::HIERARCHY_HANDLING, handling);
            }
        }
        "insideselfloopsactivate" => {
            if let Some(enabled) = parse_bool(property_value) {
                set_node_property(node, CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE, enabled);
            }
        }
        "spacingnodenode" => {
            if let Ok(spacing) = property_value.trim().parse::<f64>() {
                set_node_property(node, CoreOptions::SPACING_NODE_NODE, spacing);
            }
        }
        "spacingedgeedge" => {
            if let Ok(spacing) = property_value.trim().parse::<f64>() {
                set_node_property(node, CoreOptions::SPACING_EDGE_EDGE, spacing);
            }
        }
        "direction" => {
            if let Some(direction) = parse_direction(property_value) {
                set_node_property(node, CoreOptions::DIRECTION, direction);
            }
        }
        "edgerouting" => {
            if let Some(routing) = parse_edge_routing(property_value) {
                set_node_property(node, CoreOptions::EDGE_ROUTING, routing);
            }
        }
        "padding" => {
            if let Some(padding) = parse_padding(property_value) {
                set_node_property(node, CoreOptions::PADDING, padding);
            }
        }
        "portconstraints" => {
            if let Some(constraints) = parse_port_constraints(property_value) {
                set_node_property(node, LayeredOptions::PORT_CONSTRAINTS, constraints);
            }
        }
        "nodelabelsplacement" => {
            if let Some(placement) = parse_node_label_placement(property_value) {
                set_node_property(node, LayeredOptions::NODE_LABELS_PLACEMENT, placement);
            }
        }
        "nodelabelspadding" => {
            if let Some(padding) = parse_padding(property_value) {
                set_node_property(node, LayeredOptions::NODE_LABELS_PADDING, padding);
            }
        }
        "nodeplacementstrategy" => {
            if let Some(strategy) = parse_node_placement_strategy(property_value) {
                set_node_property(node, LayeredOptions::NODE_PLACEMENT_STRATEGY, strategy);
            }
        }
        "portlabelsplacement" => {
            if let Some(placement) = parse_port_label_placement(property_value) {
                set_node_property(node, CoreOptions::PORT_LABELS_PLACEMENT, placement);
            }
        }
        _ => {}
    }
}

fn apply_port_property(port: &ElkPortRef, property_name: &str, property_value: &str) {
    match normalize_property_key(property_name).as_str() {
        "side" | "portside" => {
            if let Some(side) = parse_port_side(property_value) {
                set_port_side_property(port, side);
            }
        }
        "portborderoffset" => {
            if let Ok(offset) = property_value.trim().parse::<f64>() {
                set_port_property(port, LayeredOptions::PORT_BORDER_OFFSET, offset);
            }
        }
        "portanchor" => {
            let values = parse_numbers(property_value);
            if values.len() >= 2 {
                set_port_property(
                    port,
                    LayeredOptions::PORT_ANCHOR,
                    KVector::with_values(values[0], values[1]),
                );
            }
        }
        _ => {}
    }
}

fn apply_edge_property(edge: &ElkEdgeRef, property_name: &str, property_value: &str) {
    match normalize_property_key(property_name).as_str() {
        "yo" | "insideselfloopsyo" => {
            if let Some(enabled) = parse_bool(property_value) {
                edge.borrow_mut()
                    .element()
                    .properties_mut()
                    .set_property(CoreOptions::INSIDE_SELF_LOOPS_YO, Some(enabled));
            }
        }
        "edgelabelplacement" | "edgelabelsplacement" => {
            if let Some(placement) = parse_edge_label_placement(property_value) {
                edge.borrow_mut()
                    .element()
                    .properties_mut()
                    .set_property(LayeredOptions::EDGE_LABELS_PLACEMENT, Some(placement));
            }
        }
        "junctionpoints" => {
            let points = parse_point_pairs(property_value);
            if !points.is_empty() {
                let mut chain = KVectorChain::new();
                for (x, y) in points {
                    chain.add_values(x, y);
                }
                edge.borrow_mut()
                    .element()
                    .properties_mut()
                    .set_property(LayeredOptions::JUNCTION_POINTS, Some(chain));
            }
        }
        "sourcepoint" => {
            apply_edge_section(edge, parse_point_pair(property_value), None, None, false);
        }
        "targetpoint" => {
            apply_edge_section(edge, None, parse_point_pair(property_value), None, false);
        }
        "bendpoints" => {
            apply_edge_section(edge, None, None, Some(parse_point_pairs(property_value)), true);
        }
        _ => {}
    }
}

fn apply_edge_section(
    edge: &ElkEdgeRef,
    source_point: Option<(f64, f64)>,
    target_point: Option<(f64, f64)>,
    bend_points: Option<Vec<(f64, f64)>>,
    replace_bends: bool,
) {
    let section = ensure_primary_edge_section(edge);
    apply_edge_section_to(&section, source_point, target_point, bend_points, replace_bends);
}

fn apply_edge_section_to(
    section: &ElkEdgeSectionRef,
    source_point: Option<(f64, f64)>,
    target_point: Option<(f64, f64)>,
    bend_points: Option<Vec<(f64, f64)>>,
    replace_bends: bool,
) {
    let mut section_mut = section.borrow_mut();

    if let Some((x, y)) = source_point {
        section_mut.set_start_x(x);
        section_mut.set_start_y(y);
    }
    if let Some((x, y)) = target_point {
        section_mut.set_end_x(x);
        section_mut.set_end_y(y);
    }

    if let Some(points) = bend_points {
        if replace_bends {
            section_mut.bend_points().clear();
        }
        for (x, y) in points {
            let bend = ElkBendPoint::new();
            bend.borrow_mut().set_x(x);
            bend.borrow_mut().set_y(y);
            section_mut.bend_points().push(bend);
        }
    }
}

fn ensure_primary_edge_section(edge: &ElkEdgeRef) -> org_eclipse_elk_graph::org::eclipse::elk::graph::ElkEdgeSectionRef {
    if let Some(section) = edge.borrow_mut().sections().get(0) {
        return section;
    }

    let section = ElkEdgeSection::new();
    ElkEdgeSection::set_parent(&section, Some(edge.clone()));
    let mut section_mut = section.borrow_mut();

    if let Some(source) = edge.borrow_mut().sources().get(0) {
        section_mut.set_outgoing_shape(Some(source));
    }
    if let Some(target) = edge.borrow_mut().targets().get(0) {
        section_mut.set_incoming_shape(Some(target));
    }

    drop(section_mut);
    section
}

fn get_or_create_edge_section_for_identifier(
    edge: &ElkEdgeRef,
    section_map: &mut HashMap<String, ElkEdgeSectionRef>,
    section_identifier: &str,
) -> ElkEdgeSectionRef {
    if let Some(section) = section_map.get(section_identifier) {
        return section.clone();
    }

    let section = ElkEdgeSection::new();
    ElkEdgeSection::set_parent(&section, Some(edge.clone()));
    section
        .borrow_mut()
        .set_identifier(Some(section_identifier.to_string()));
    section_map.insert(section_identifier.to_string(), section.clone());
    section
}

fn resolve_pending_section_links(
    edge_sections_by_edge: &HashMap<String, HashMap<String, ElkEdgeSectionRef>>,
    pending_section_links: &[PendingSectionLink],
) -> Result<(), String> {
    for link in pending_section_links {
        let Some(section_map) = edge_sections_by_edge.get(link.edge_id.as_str()) else {
            return Err(line_context_error(
                link.line_number,
                link.line_text.as_str(),
                format!(
                    "section link references unknown edge '{}' for source section '{}'",
                    link.edge_id, link.source_section_id
                ),
            ));
        };
        let Some(source_section) = section_map.get(link.source_section_id.as_str()) else {
            return Err(line_context_error(
                link.line_number,
                link.line_text.as_str(),
                format!(
                    "section link references unknown source section '{}' in edge '{}'",
                    link.source_section_id, link.edge_id
                ),
            ));
        };

        let mut outgoing_sections = source_section.borrow_mut().outgoing_sections();
        for target_id in &link.target_section_ids {
            let Some(target_section) = section_map.get(target_id) else {
                return Err(line_context_error(
                    link.line_number,
                    link.line_text.as_str(),
                    format!(
                        "section link references unknown target section '{target_id}' from source section '{}' in edge '{}'",
                        link.source_section_id, link.edge_id
                    ),
                ));
            };
            if !outgoing_sections
                .iter()
                .any(|section| std::rc::Rc::ptr_eq(section, target_section))
            {
                outgoing_sections.push(target_section.clone());
            }

            let mut target_mut = target_section.borrow_mut();
            let mut incoming_sections = target_mut.incoming_sections();
            if !incoming_sections
                .iter()
                .any(|section| std::rc::Rc::ptr_eq(section, source_section))
            {
                incoming_sections.push(source_section.clone());
                target_mut.set_incoming_sections(incoming_sections);
            }
        }

        source_section
            .borrow_mut()
            .set_outgoing_sections(outgoing_sections);
    }

    Ok(())
}

fn register_label_identifier(
    label_identifiers: &mut HashSet<String>,
    scope_key: &str,
    identifier: &str,
) -> Result<(), String> {
    let scoped = format!("{scope_key}::{identifier}");
    if label_identifiers.insert(scoped) {
        Ok(())
    } else {
        Err(format!("duplicate label identifier: {identifier}"))
    }
}

fn line_context_error<M: AsRef<str>>(line_number: usize, line: &str, message: M) -> String {
    let line = line.trim();
    if line.is_empty() {
        format!("line {line_number}: {}", message.as_ref())
    } else {
        format!("line {line_number}: {} | {line}", message.as_ref())
    }
}

fn normalize_key(key: &str) -> String {
    key.trim()
        .to_ascii_lowercase()
        .replace(['_', '-', ' ', '.', '^'], "")
}

fn normalize_property_key(key: &str) -> String {
    let normalized = normalize_key(key);
    if let Some(stripped) = normalized.strip_prefix("orgeclipseelklayered") {
        if !stripped.is_empty() {
            return stripped.to_string();
        }
    }
    if let Some(stripped) = normalized.strip_prefix("orgeclipseelk") {
        if !stripped.is_empty() {
            return stripped.to_string();
        }
    }
    normalized
}

fn property_key_matches(lhs: &str, rhs: &str) -> bool {
    let lhs = normalize_property_key(lhs);
    let rhs = normalize_property_key(rhs);
    lhs == rhs || lhs.ends_with(rhs.as_str())
}

fn parse_padding(value: &str) -> Option<ElkPadding> {
    let numbers = parse_numbers(value);
    match numbers.as_slice() {
        [all] => Some(ElkPadding::with_any(*all)),
        [top, left, bottom, right] => Some(ElkPadding::with_values(*top, *left, *bottom, *right)),
        _ => None,
    }
}

fn parse_numbers(value: &str) -> Vec<f64> {
    value
        .split([',', '[', ']', '{', '}', '(', ')', ';', ' '])
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .filter_map(|token| token.parse::<f64>().ok())
        .collect::<Vec<_>>()
}

fn parse_quoted_token(input: &str) -> Option<(String, usize)> {
    let mut chars = input.char_indices();
    let (_, quote) = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let mut escaped = false;
    let mut decoded = String::new();
    for (index, ch) in chars {
        if escaped {
            let unescaped = match ch {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '"' => '"',
                '\'' => '\'',
                other => other,
            };
            decoded.push(unescaped);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Some((decoded, index + ch.len_utf8()));
        }
        decoded.push(ch);
    }

    None
}

fn parse_identifier_token(token: &str) -> String {
    let trimmed = token.trim();
    if let Some((quoted, consumed)) = parse_quoted_token(trimmed) {
        if consumed == trimmed.len() {
            return quoted;
        }
    }
    trim_inline_token(trimmed).to_string()
}

fn split_once_outside_quotes(input: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for (index, ch) in input.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }

        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }

        if ch == delimiter {
            let next_index = index + ch.len_utf8();
            return Some((&input[..index], &input[next_index..]));
        }
    }

    None
}

fn split_csv_outside_quotes(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for (index, ch) in value.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }

        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }

        if ch == ',' {
            parts.push(&value[start..index]);
            start = index + 1;
        }
    }
    parts.push(&value[start..]);
    parts
}

fn split_tokens_outside_quotes(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in value.chars() {
        if let Some(active_quote) = quote {
            if escaped {
                let unescaped = match ch {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '"' => '"',
                    '\'' => '\'',
                    other => other,
                };
                current.push(unescaped);
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
                continue;
            }
            current.push(ch);
            continue;
        }

        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(current);
                current = String::new();
            }
            continue;
        }
        current.push(ch);
    }

    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_text_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((quoted, _)) = parse_quoted_token(trimmed) {
        return Some(quoted);
    }
    let token = trimmed.split_whitespace().next()?;
    Some(parse_identifier_token(token))
}

fn parse_port_constraints(value: &str) -> Option<PortConstraints> {
    match normalize_key(value).as_str() {
        "undefined" => Some(PortConstraints::Undefined),
        "free" => Some(PortConstraints::Free),
        "fixedside" => Some(PortConstraints::FixedSide),
        "fixedorder" => Some(PortConstraints::FixedOrder),
        "fixedratio" => Some(PortConstraints::FixedRatio),
        "fixedpos" => Some(PortConstraints::FixedPos),
        _ => None,
    }
}

fn parse_hierarchy_handling(value: &str) -> Option<HierarchyHandling> {
    match normalize_key(value).as_str() {
        "inherit" => Some(HierarchyHandling::Inherit),
        "includechildren" => Some(HierarchyHandling::IncludeChildren),
        "separatechildren" => Some(HierarchyHandling::SeparateChildren),
        _ => None,
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match normalize_key(value).as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn parse_port_side(value: &str) -> Option<PortSide> {
    match normalize_key(value).as_str() {
        "north" | "up" => Some(PortSide::North),
        "east" | "right" => Some(PortSide::East),
        "south" | "down" => Some(PortSide::South),
        "west" | "left" => Some(PortSide::West),
        "undefined" => Some(PortSide::Undefined),
        _ => None,
    }
}

fn parse_node_label_placement(value: &str) -> Option<EnumSet<NodeLabelPlacement>> {
    match normalize_key(value).as_str() {
        "fixed" => return Some(NodeLabelPlacement::fixed()),
        "insidetopleft" => return Some(NodeLabelPlacement::inside_top_left()),
        "insidetopcenter" => return Some(NodeLabelPlacement::inside_top_center()),
        "insidetopright" => return Some(NodeLabelPlacement::inside_top_right()),
        "insidecenter" => return Some(NodeLabelPlacement::inside_center()),
        "insidebottomleft" => return Some(NodeLabelPlacement::inside_bottom_left()),
        "insidebottomcenter" => return Some(NodeLabelPlacement::inside_bottom_center()),
        "insidebottomright" => return Some(NodeLabelPlacement::inside_bottom_right()),
        "outsidetopleft" => return Some(NodeLabelPlacement::outside_top_left()),
        "outsidetopcenter" => return Some(NodeLabelPlacement::outside_top_center()),
        "outsidetopright" => return Some(NodeLabelPlacement::outside_top_right()),
        "outsidebottomleft" => return Some(NodeLabelPlacement::outside_bottom_left()),
        "outsidebottomcenter" => return Some(NodeLabelPlacement::outside_bottom_center()),
        "outsidebottomright" => return Some(NodeLabelPlacement::outside_bottom_right()),
        _ => {}
    }

    let placements = value
        .split([',', '[', ']', '{', '}', ' '])
        .map(normalize_key)
        .filter_map(|token| match token.as_str() {
            "inside" => Some(NodeLabelPlacement::Inside),
            "outside" => Some(NodeLabelPlacement::Outside),
            "hleft" => Some(NodeLabelPlacement::HLeft),
            "hcenter" => Some(NodeLabelPlacement::HCenter),
            "hright" => Some(NodeLabelPlacement::HRight),
            "vtop" => Some(NodeLabelPlacement::VTop),
            "vcenter" => Some(NodeLabelPlacement::VCenter),
            "vbottom" => Some(NodeLabelPlacement::VBottom),
            "hpriority" => Some(NodeLabelPlacement::HPriority),
            _ => None,
        })
        .collect::<Vec<_>>();

    if placements.is_empty() {
        None
    } else {
        Some(EnumSet::of(&placements))
    }
}

fn parse_node_placement_strategy(value: &str) -> Option<NodePlacementStrategy> {
    match normalize_key(value).as_str() {
        "simple" => Some(NodePlacementStrategy::Simple),
        "interactive" => Some(NodePlacementStrategy::Interactive),
        "linearsegments" => Some(NodePlacementStrategy::LinearSegments),
        "brandeskoepf" => Some(NodePlacementStrategy::BrandesKoepf),
        "networksimplex" => Some(NodePlacementStrategy::NetworkSimplex),
        _ => None,
    }
}

fn parse_port_label_placement(value: &str) -> Option<EnumSet<PortLabelPlacement>> {
    if normalize_key(value).as_str() == "fixed" {
        return Some(PortLabelPlacement::fixed());
    }

    let placements = value
        .split([',', '[', ']', '{', '}', ' '])
        .map(normalize_key)
        .filter_map(|token| match token.as_str() {
            "inside" => Some(PortLabelPlacement::Inside),
            "outside" => Some(PortLabelPlacement::Outside),
            "nexttoportifpossible" => Some(PortLabelPlacement::NextToPortIfPossible),
            "alwayssameside" => Some(PortLabelPlacement::AlwaysSameSide),
            "alwaysothersameside" => Some(PortLabelPlacement::AlwaysOtherSameSide),
            "spaceefficient" => Some(PortLabelPlacement::SpaceEfficient),
            _ => None,
        })
        .collect::<Vec<_>>();

    if placements.is_empty() {
        None
    } else {
        Some(EnumSet::of(&placements))
    }
}

fn parse_edge_label_placement(value: &str) -> Option<EdgeLabelPlacement> {
    match normalize_key(value).as_str() {
        "head" => Some(EdgeLabelPlacement::Head),
        "tail" => Some(EdgeLabelPlacement::Tail),
        "center" | "middle" => Some(EdgeLabelPlacement::Center),
        _ => None,
    }
}

fn parse_property_line(line: &str) -> Option<(&str, &str)> {
    split_key_value_once(line)
}

fn parse_inline_field_value(line: &str, key: &str) -> Option<String> {
    if let Some((_, value_start)) = find_key_marker(line, key) {
        let tail = &line[value_start..];
        return parse_inline_token_value(tail);
    }

    for token in line.split_whitespace() {
        let Some((lhs, rhs)) = split_inline_token(token) else {
            continue;
        };
        if !rhs.is_empty() && property_key_matches(lhs, key) {
            return Some(trim_inline_token(rhs).to_string());
        }
    }
    None
}

fn parse_named_value<'a>(line: &'a str, key: &str, next_keys: &[&str]) -> Option<&'a str> {
    let (_, start) = find_key_marker(line, key)?;
    let tail = &line[start..];
    let mut end = tail.len();

    for next_key in next_keys {
        if let Some((index, _)) = find_key_marker(tail, next_key) {
            end = end.min(index);
        }
    }

    let value = tail[..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn find_key_marker(line: &str, key: &str) -> Option<(usize, usize)> {
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for (marker_start, ch) in line.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }

        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }

        if !line[marker_start..].starts_with(key) {
            continue;
        }

        let previous = line[..marker_start].chars().next_back();
        let starts_at_boundary = previous
            .map(|prev| prev.is_ascii_whitespace() || matches!(prev, '[' | '{' | '(' | ',' | '|' | ';'))
            .unwrap_or(true);
        if !starts_at_boundary {
            continue;
        }

        let mut delimiter_index = marker_start + key.len();
        while let Some(next_ch) = line[delimiter_index..].chars().next() {
            if !next_ch.is_ascii_whitespace() {
                break;
            }
            delimiter_index += next_ch.len_utf8();
        }
        let Some(delimiter) = line[delimiter_index..].chars().next() else {
            continue;
        };
        if !matches!(delimiter, ':' | '=') {
            continue;
        }

        delimiter_index += delimiter.len_utf8();
        while let Some(next_ch) = line[delimiter_index..].chars().next() {
            if !next_ch.is_ascii_whitespace() {
                break;
            }
            delimiter_index += next_ch.len_utf8();
        }
        return Some((marker_start, delimiter_index));
    }

    None
}

fn parse_point_pair(value: &str) -> Option<(f64, f64)> {
    let numbers = parse_numbers(value);
    if numbers.len() >= 2 {
        Some((numbers[0], numbers[1]))
    } else {
        None
    }
}

fn parse_point_pairs(value: &str) -> Vec<(f64, f64)> {
    if value.contains(';') || value.contains('|') {
        return value
            .split([';', '|'])
            .filter_map(parse_point_pair)
            .collect();
    }

    let numbers = parse_numbers(value);
    let mut points = Vec::new();
    let mut index = 0;
    while index + 1 < numbers.len() {
        points.push((numbers[index], numbers[index + 1]));
        index += 2;
    }
    points
}

fn parse_edge_section_declaration(line: &str) -> Option<EdgePointDeclaration> {
    if !line.starts_with("edgeSection ") {
        return None;
    }

    let rest = line.trim_start_matches("edgeSection").trim();
    let (edge_id, remaining) = split_first_token(rest)?;
    let edge_id = parse_identifier_token(edge_id);
    let source_point = parse_named_value_any(
        remaining,
        &["sourcePoint", "source", "start"],
        &["targetPoint", "target", "end", "bendPoints", "bendPoint", "bend", "bends"],
    )
    .and_then(parse_point_pair);
    let target_point = parse_named_value_any(
        remaining,
        &["targetPoint", "target", "end"],
        &["sourcePoint", "source", "start", "bendPoints", "bendPoint", "bend", "bends"],
    )
    .and_then(parse_point_pair);
    let bend_points = parse_named_value_any(
        remaining,
        &["bendPoints", "bendPoint", "bend", "bends"],
        &["sourcePoint", "source", "start", "targetPoint", "target", "end"],
    )
    .map(parse_point_pairs);

    Some((edge_id, source_point, target_point, bend_points))
}

fn parse_edge_point_declaration(line: &str) -> Option<EdgePointDeclaration> {
    if !line.starts_with("edgePoint ") {
        return None;
    }

    let rest = line.trim_start_matches("edgePoint").trim();
    let (edge_id, remaining) = split_first_token(rest)?;
    let edge_id = parse_identifier_token(edge_id);
    let source_point = parse_named_value(
        remaining,
        "source",
        &["target", "bend", "sourcePoint", "targetPoint", "bendPoint", "bendPoints"],
    )
    .and_then(parse_point_pair)
    .or_else(|| {
        parse_named_value(
            remaining,
            "sourcePoint",
            &["target", "bend", "source", "targetPoint", "bendPoint", "bendPoints"],
        )
        .and_then(parse_point_pair)
    });
    let target_point = parse_named_value(
        remaining,
        "target",
        &["source", "bend", "sourcePoint", "targetPoint", "bendPoint", "bendPoints"],
    )
    .and_then(parse_point_pair)
    .or_else(|| {
        parse_named_value(
            remaining,
            "targetPoint",
            &["source", "bend", "sourcePoint", "target", "bendPoint", "bendPoints"],
        )
        .and_then(parse_point_pair)
    });
    let bend_points = parse_named_value(
        remaining,
        "bend",
        &["source", "target", "sourcePoint", "targetPoint", "bendPoint", "bendPoints"],
    )
    .map(parse_point_pairs)
    .or_else(|| {
        parse_named_value(
            remaining,
            "bendPoint",
            &["source", "target", "sourcePoint", "targetPoint", "bend", "bendPoints"],
        )
        .map(parse_point_pairs)
    })
    .or_else(|| {
        parse_named_value(
            remaining,
            "bendPoints",
            &["source", "target", "sourcePoint", "targetPoint", "bend", "bendPoint"],
        )
        .map(parse_point_pairs)
    });

    Some((edge_id, source_point, target_point, bend_points))
}

fn parse_section_declaration(line: &str) -> Option<SectionDeclaration> {
    if !line.starts_with("section ") {
        return None;
    }

    let rest = line.trim_start_matches("section").trim();
    let (section_id, remaining) = split_first_token(rest)?;
    let section_id = parse_identifier_token(section_id);

    let mut outgoing_section_ids = Vec::new();
    let mut additional_outgoing_links = Vec::new();
    let mut link_has_empty_target = false;
    let mut layout_part = remaining.trim().to_string();
    if let Some(after_arrow) = remaining.trim().strip_prefix("->") {
        let after_arrow = after_arrow.trim();
        let (outgoing_part, rest_part) = split_section_outgoing_and_layout(after_arrow);
        let (first_targets, extra_links, has_empty_target) =
            parse_section_link_steps(outgoing_part.as_str());
        outgoing_section_ids = first_targets;
        additional_outgoing_links = extra_links;
        link_has_empty_target = has_empty_target;
        layout_part = rest_part.trim().to_string();
    } else if let Some(arrow_index) = find_arrow_outside_groups(remaining.trim()) {
        let before_arrow = remaining.trim()[..arrow_index].trim();
        let after_arrow = remaining.trim()[arrow_index + 2..].trim();
        let (first_targets, extra_links, has_empty_target) = parse_section_link_steps(after_arrow);
        outgoing_section_ids = first_targets;
        additional_outgoing_links = extra_links;
        link_has_empty_target = has_empty_target;
        layout_part = before_arrow.to_string();
    }

    let section_layout = extract_layout_body(layout_part.as_str());

    let incoming_shape_id = parse_named_value_any(
        section_layout,
        &["incoming", "incomingShape", "incomingShapeId"],
        &[
            "outgoing",
            "outgoingShape",
            "outgoingShapeId",
            "start",
            "source",
            "sourcePoint",
            "end",
            "target",
            "targetPoint",
            "bends",
            "bend",
            "bendPoint",
            "bendPoints",
        ],
    )
    .and_then(parse_identifier_value);
    let outgoing_shape_id = parse_named_value_any(
        section_layout,
        &["outgoing", "outgoingShape", "outgoingShapeId"],
        &[
            "incoming",
            "incomingShape",
            "incomingShapeId",
            "start",
            "source",
            "sourcePoint",
            "end",
            "target",
            "targetPoint",
            "bends",
            "bend",
            "bendPoint",
            "bendPoints",
        ],
    )
    .and_then(parse_identifier_value);
    let source_point = parse_named_value_any(
        section_layout,
        &["start", "source", "sourcePoint"],
        &[
            "incoming",
            "incomingShape",
            "incomingShapeId",
            "outgoing",
            "outgoingShape",
            "outgoingShapeId",
            "end",
            "target",
            "targetPoint",
            "bends",
            "bend",
            "bendPoint",
            "bendPoints",
        ],
    )
    .and_then(parse_point_pair);
    let target_point = parse_named_value_any(
        section_layout,
        &["end", "target", "targetPoint"],
        &[
            "incoming",
            "incomingShape",
            "incomingShapeId",
            "outgoing",
            "outgoingShape",
            "outgoingShapeId",
            "start",
            "source",
            "sourcePoint",
            "bends",
            "bend",
            "bendPoint",
            "bendPoints",
        ],
    )
    .and_then(parse_point_pair);
    let bend_points = parse_named_value_any(
        section_layout,
        &["bends", "bend", "bendPoint", "bendPoints"],
        &[
            "incoming",
            "incomingShape",
            "incomingShapeId",
            "outgoing",
            "outgoingShape",
            "outgoingShapeId",
            "start",
            "source",
            "sourcePoint",
            "end",
            "target",
            "targetPoint",
        ],
    )
    .map(parse_point_pairs);

    Some(SectionDeclaration {
        id: section_id,
        outgoing_section_ids,
        additional_outgoing_links,
        link_has_empty_target,
        incoming_shape_id,
        outgoing_shape_id,
        geometry: (source_point, target_point, bend_points),
    })
}

fn parse_named_value_any<'a>(
    line: &'a str,
    keys: &[&str],
    next_keys: &[&str],
) -> Option<&'a str> {
    for key in keys {
        if let Some(value) = parse_named_value(line, key, next_keys) {
            return Some(value);
        }
    }
    None
}

fn parse_identifier_list(value: &str) -> Vec<String> {
    split_csv_outside_quotes(value)
        .into_iter()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(parse_identifier_token)
        .collect()
}

fn parse_identifier_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((quoted, _)) = parse_quoted_token(trimmed) {
        return Some(quoted);
    }
    let token = trimmed
        .split([' ', '\t', ',', ';', '|', ']', ')', '}'])
        .find(|part| !part.is_empty())?;
    Some(parse_identifier_token(token))
}

fn parse_inline_value(line: &str, key: &str) -> Option<String> {
    if let Some((_, value_start)) = find_key_marker(line, key) {
        let tail = &line[value_start..];
        return parse_inline_token_value(tail);
    }

    for token in line.split_whitespace() {
        let Some((lhs, rhs)) = split_inline_token(token) else {
            continue;
        };
        if !rhs.is_empty() && property_key_matches(lhs, key) {
            return Some(trim_inline_token(rhs).to_string());
        }
    }
    None
}

fn trim_inline_token(value: &str) -> &str {
    value
        .trim()
        .trim_matches(|ch| matches!(ch, '"' | '\'' | ',' | ';' | ')' | ']' | '}'))
}

fn parse_inline_token_value(tail: &str) -> Option<String> {
    let trimmed = tail.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((quoted, _)) = parse_quoted_token(trimmed) {
        return Some(quoted);
    }
    let token = trimmed
        .split([' ', '\t', ',', ';', '}', ']', ')'])
        .find(|part| !part.is_empty())?;
    Some(trim_inline_token(token).to_string())
}

fn split_section_outgoing_and_layout(input: &str) -> (String, String) {
    if let Some(split_index) = input.find(['[', '(']) {
        return (
            input[..split_index].to_string(),
            input[split_index..].to_string(),
        );
    }

    let tokens = input.split_whitespace().collect::<Vec<_>>();
    let split_at = tokens
        .iter()
        .position(|token| token.contains(':') || token.contains('='))
        .unwrap_or(tokens.len());
    (tokens[..split_at].join(" "), tokens[split_at..].join(" "))
}

fn parse_section_link_steps(raw_links: &str) -> SectionLinkParseResult {
    let raw_steps = raw_links
        .split("->")
        .map(str::trim)
        .collect::<Vec<_>>();

    let mut has_empty_target = raw_steps.is_empty() || raw_steps.iter().any(|step| step.is_empty());
    let parsed_steps = raw_steps
        .iter()
        .map(|step| parse_identifier_list(step))
        .collect::<Vec<_>>();
    if parsed_steps.is_empty() || parsed_steps.iter().any(|ids| ids.is_empty()) {
        has_empty_target = true;
    }

    let link_steps = parsed_steps
        .into_iter()
        .filter(|ids| !ids.is_empty())
        .collect::<Vec<_>>();
    let first_targets = link_steps
        .first()
        .cloned()
        .unwrap_or_default();

    let mut additional_outgoing_links = Vec::new();
    for pair in link_steps.windows(2) {
        let sources = &pair[0];
        let targets = pair[1].clone();
        for source in sources {
            additional_outgoing_links.push((source.clone(), targets.clone()));
        }
    }

    (first_targets, additional_outgoing_links, has_empty_target)
}

fn find_arrow_outside_groups(input: &str) -> Option<usize> {
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let bytes = input.as_bytes();
    let mut index = 0usize;

    while index + 1 < bytes.len() {
        match bytes[index] as char {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            _ => {}
        }

        if bytes[index] == b'-'
            && bytes[index + 1] == b'>'
            && bracket_depth == 0
            && paren_depth == 0
            && brace_depth == 0
        {
            return Some(index);
        }

        index += 1;
    }
    None
}

fn extract_layout_body(layout_part: &str) -> &str {
    let bracket_index = layout_part.find('[');
    let paren_index = layout_part.find('(');
    let open = match (bracket_index, paren_index) {
        (Some(b), Some(p)) if b <= p => Some((b, ']')),
        (Some(_), Some(p)) => Some((p, ')')),
        (Some(b), None) => Some((b, ']')),
        (None, Some(p)) => Some((p, ')')),
        (None, None) => None,
    };

    if let Some((start_index, close_delim)) = open {
        let after = &layout_part[start_index + 1..];
        if let Some(end_index) = after.rfind(close_delim) {
            &after[..end_index]
        } else {
            after
        }
    } else {
        layout_part
    }
}

fn parse_entity_property<'a>(
    line: &'a str,
    prefix: &str,
) -> Option<(String, &'a str, &'a str)> {
    if !starts_with_statement(line, prefix) {
        return None;
    }

    let rest = line.trim_start_matches(prefix).trim();
    let (entity_id, property) = split_first_token(rest)?;
    let (name, value) = split_key_value_once(property)?;
    Some((parse_identifier_token(entity_id), name.trim(), value.trim()))
}

fn split_first_token(input: &str) -> Option<(&str, &str)> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    if input.starts_with('"') || input.starts_with('\'') {
        let (_, consumed) = parse_quoted_token(input)?;
        let token = input[..consumed].trim();
        let rest = input[consumed..].trim();
        return Some((token, rest));
    }

    let split_index = input.find(char::is_whitespace).unwrap_or(input.len());
    let token = input[..split_index].trim();
    let rest = input[split_index..].trim();
    Some((token, rest))
}

fn parse_port_declaration(line: &str) -> Option<PortDeclaration> {
    if !line.starts_with("port ") {
        return None;
    }

    let rest = line.trim_start_matches("port").trim();
    let has_block = rest.contains('{');
    let declaration = rest
        .split(['{', '['])
        .next()
        .map(str::trim)
        .unwrap_or(rest);
    let tokens = split_tokens_outside_quotes(declaration);
    let port_id = tokens.first()?.clone();
    let node_id = parse_inline_field_value(declaration, "of")
        .map(|value| parse_identifier_token(value.as_str()))
        .filter(|value| !value.is_empty())
        .or_else(|| find_relation_reference(&tokens, &["of"]));

    Some(PortDeclaration {
        id: parse_identifier_token(port_id.as_str()),
        node_id,
        has_block,
    })
}

fn parse_node_declaration(line: &str) -> Option<NodeDeclaration> {
    if !line.starts_with("node ") {
        return None;
    }

    let rest = line.trim_start_matches("node").trim();
    let has_block = rest.contains('{');
    let declaration = rest
        .split(['{', '['])
        .next()
        .map(str::trim)
        .unwrap_or(rest);
    let tokens = split_tokens_outside_quotes(declaration);
    let id = tokens.first()?.clone();

    let parent_id = parse_inline_field_value(declaration, "parent")
        .or_else(|| parse_inline_field_value(declaration, "in"))
        .or_else(|| parse_inline_field_value(declaration, "of"))
        .map(|value| parse_identifier_token(value.as_str()))
        .filter(|value| !value.is_empty())
        .or_else(|| find_relation_reference(&tokens, &["parent", "in", "of"]));

    Some(NodeDeclaration {
        id: parse_identifier_token(id.as_str()),
        parent_id,
        has_block,
    })
}

fn find_relation_reference(tokens: &[String], keys: &[&str]) -> Option<String> {
    if tokens.len() < 2 {
        return None;
    }

    for index in 1..tokens.len().saturating_sub(1) {
        let token = normalize_key(tokens[index].as_str());
        if keys.iter().any(|key| token == *key) {
            let identifier = parse_identifier_token(tokens[index + 1].as_str());
            if !identifier.is_empty() {
                return Some(identifier);
            }
        }
    }

    None
}

fn parse_edge_declaration(line: &str) -> Option<EdgeDeclaration> {
    if !line.starts_with("edge ") {
        return None;
    }

    let rest = line.trim_start_matches("edge").trim();
    let has_block = rest.contains('{');
    let declaration = rest.split('{').next().map(str::trim).unwrap_or(rest);
    let (left, right) = declaration.split_once("->")?;
    let target = right.trim();
    let left = left.trim();
    if target.is_empty() || left.is_empty() {
        return None;
    }

    if let Some((edge_id, source)) = split_once_outside_quotes(left, ':') {
        let edge_id = parse_identifier_token(edge_id);
        let source = source.trim();
        let source_ids = parse_identifier_list(source);
        let target_ids = parse_identifier_list(target);
        if !edge_id.is_empty() && !source_ids.is_empty() && !target_ids.is_empty() {
            return Some(EdgeDeclaration {
                id: Some(edge_id),
                source_ids,
                target_ids,
                has_block,
            });
        }
    }

    let source_ids = parse_identifier_list(left);
    let target_ids = parse_identifier_list(target);
    if source_ids.is_empty() || target_ids.is_empty() {
        return None;
    }

    Some(EdgeDeclaration {
        id: None,
        source_ids,
        target_ids,
        has_block,
    })
}

fn parse_edge_label_declaration(line: &str) -> Option<LabelDeclaration> {
    if !line.starts_with("edgeLabel ") {
        return None;
    }

    let rest = line.trim_start_matches("edgeLabel").trim();
    let (edge_id, remaining) = split_first_token(rest)?;
    let edge_id = parse_identifier_token(edge_id);
    let text = parse_label_text(remaining).unwrap_or_else(|| "label".to_string());
    let size = parse_size(remaining);
    Some((edge_id, text, size))
}

fn parse_node_label_declaration(line: &str) -> Option<LabelDeclaration> {
    if !line.starts_with("nodeLabel ") {
        return None;
    }

    let rest = line.trim_start_matches("nodeLabel").trim();
    let (node_id, remaining) = split_first_token(rest)?;
    let node_id = parse_identifier_token(node_id);
    let text = parse_label_text(remaining).unwrap_or_else(|| "label".to_string());
    let size = parse_size(remaining);
    Some((node_id, text, size))
}

fn parse_port_label_declaration(line: &str) -> Option<LabelDeclaration> {
    if !line.starts_with("portLabel ") {
        return None;
    }

    let rest = line.trim_start_matches("portLabel").trim();
    let (port_id, remaining) = split_first_token(rest)?;
    let port_id = parse_identifier_token(port_id);
    let text = parse_label_text(remaining).unwrap_or_else(|| "label".to_string());
    let size = parse_size(remaining);
    Some((port_id, text, size))
}

fn parse_label_declaration(line: &str) -> Option<GenericLabelDeclaration> {
    if !line.starts_with("label ") {
        return None;
    }

    let rest = line.trim_start_matches("label").trim();
    let has_block = rest.contains('{');
    let declaration = rest.split('{').next().map(str::trim).unwrap_or(rest);
    let (identifier, declaration_without_identifier) = parse_label_identifier_prefix(declaration);

    let text = parse_label_text(declaration_without_identifier).unwrap_or_else(|| "label".to_string());
    let size = parse_size(declaration_without_identifier);
    let edge_label_placement = parse_inline_field_value(declaration_without_identifier, "placement")
        .or_else(|| parse_inline_field_value(declaration_without_identifier, "edgeLabelPlacement"))
        .and_then(|value| parse_edge_label_placement(value.as_str()));

    Some(GenericLabelDeclaration {
        identifier,
        text,
        size,
        edge_label_placement,
        has_block,
    })
}

fn parse_label_identifier_prefix(declaration: &str) -> (Option<String>, &str) {
    let trimmed = declaration.trim();
    let Some((prefix, suffix)) = split_once_outside_quotes(trimmed, ':') else {
        return (None, trimmed);
    };

    let prefix = prefix.trim();
    let suffix = suffix.trim_start();
    if prefix.is_empty() {
        return (None, trimmed);
    }
    if !prefix.starts_with('"') && !prefix.starts_with('\'') && prefix.contains(char::is_whitespace) {
        return (None, trimmed);
    }
    let text_key = split_key_value_once(suffix)
        .map(|(name, _)| normalize_key(name))
        .is_some_and(|name| name == "text");
    if !suffix.starts_with('"') && !suffix.starts_with('\'') && !text_key {
        return (None, trimmed);
    }

    (Some(parse_identifier_token(prefix)), suffix)
}

fn parse_label_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(text_value) = parse_named_value(trimmed, "text", &["size"]) {
        return parse_text_value(text_value);
    }

    if let Some((quoted, _)) = parse_quoted_token(trimmed) {
        return Some(quoted);
    }

    let token = trimmed.split_whitespace().next()?;
    Some(parse_identifier_token(token))
}

pub fn find_node_by_identifier(graph: &ElkNodeRef, identifier: &str) -> Option<ElkNodeRef> {
    let mut queue: VecDeque<ElkNodeRef> = VecDeque::new();
    queue.push_back(graph.clone());

    while let Some(node) = queue.pop_front() {
        let matches = {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .identifier()
                == Some(identifier)
        };
        if matches {
            return Some(node);
        }

        let children: Vec<_> = node.borrow_mut().children().iter().cloned().collect();
        queue.extend(children);
    }

    None
}

pub fn find_edge_by_identifier(
    graph: &ElkNodeRef,
    source_identifier: &str,
    target_identifier: &str,
) -> Option<ElkEdgeRef> {
    let mut queue: VecDeque<ElkNodeRef> = VecDeque::new();
    queue.push_back(graph.clone());

    while let Some(node) = queue.pop_front() {
        let edges: Vec<_> = node.borrow_mut().contained_edges().iter().cloned().collect();
        for edge in edges {
            let mut edge_mut = edge.borrow_mut();
            let sources = edge_mut
                .sources()
                .iter()
                .filter_map(extract_connectable_identifier)
                .collect::<Vec<_>>();
            let targets = edge_mut
                .targets()
                .iter()
                .filter_map(extract_connectable_identifier)
                .collect::<Vec<_>>();
            if sources.iter().any(|source| source == source_identifier)
                && targets.iter().any(|target| target == target_identifier)
            {
                return Some(edge.clone());
            }
        }

        let children: Vec<_> = node.borrow_mut().children().iter().cloned().collect();
        queue.extend(children);
    }

    None
}

pub fn find_port_by_identifier(graph: &ElkNodeRef, identifier: &str) -> Option<ElkPortRef> {
    let mut queue: VecDeque<ElkNodeRef> = VecDeque::new();
    queue.push_back(graph.clone());

    while let Some(node) = queue.pop_front() {
        for port in node.borrow_mut().ports().iter() {
            let matches = {
                let mut port_mut = port.borrow_mut();
                port_mut
                    .connectable()
                    .shape()
                    .graph_element()
                    .identifier()
                    == Some(identifier)
            };
            if matches {
                return Some(port.clone());
            }
        }

        let children: Vec<_> = node.borrow_mut().children().iter().cloned().collect();
        queue.extend(children);
    }

    None
}

pub fn find_label_by_identifier(graph: &ElkNodeRef, identifier: &str) -> Option<ElkLabelRef> {
    let mut queue: VecDeque<ElkNodeRef> = VecDeque::new();
    queue.push_back(graph.clone());

    while let Some(node) = queue.pop_front() {
        let node_labels: Vec<_> = node
            .borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .labels()
            .iter()
            .cloned()
            .collect();
        if let Some(label) = find_label_in_tree(&node_labels, identifier) {
            return Some(label);
        }

        let ports: Vec<_> = node.borrow_mut().ports().iter().cloned().collect();
        for port in ports {
            let port_labels: Vec<_> = port
                .borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .labels()
                .iter()
                .cloned()
                .collect();
            if let Some(label) = find_label_in_tree(&port_labels, identifier) {
                return Some(label);
            }
        }

        let edges: Vec<_> = node.borrow_mut().contained_edges().iter().cloned().collect();
        for edge in edges {
            let edge_labels: Vec<_> = edge.borrow_mut().element().labels().iter().cloned().collect();
            if let Some(label) = find_label_in_tree(&edge_labels, identifier) {
                return Some(label);
            }
        }

        let children: Vec<_> = node.borrow_mut().children().iter().cloned().collect();
        queue.extend(children);
    }

    None
}

fn find_label_in_tree(labels: &[ElkLabelRef], identifier: &str) -> Option<ElkLabelRef> {
    for label in labels {
        let matches = {
            let mut label_mut = label.borrow_mut();
            label_mut.shape().graph_element().identifier() == Some(identifier)
        };
        if matches {
            return Some(label.clone());
        }

        let nested_labels: Vec<_> = label
            .borrow_mut()
            .shape()
            .graph_element()
            .labels()
            .iter()
            .cloned()
            .collect();
        if let Some(found) = find_label_in_tree(&nested_labels, identifier) {
            return Some(found);
        }
    }

    None
}

fn extract_connectable_identifier(shape: &ElkConnectableShapeRef) -> Option<String> {
    match shape {
        ElkConnectableShapeRef::Node(node) => node
            .borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .identifier()
            .map(ToString::to_string),
        ElkConnectableShapeRef::Port(port) => port
            .borrow_mut()
            .connectable()
            .shape()
            .graph_element()
            .identifier()
            .map(ToString::to_string),
    }
}

fn parse_key_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let (lhs, rhs) = split_key_value_once(line)?;
    if property_key_matches(lhs, key) {
        Some(rhs.trim())
    } else {
        None
    }
}

fn parse_position(line: &str) -> Option<(f64, f64)> {
    parse_named_value_any(line, &["position"], &["size"]).and_then(parse_point_pair)
}

fn parse_size(line: &str) -> Option<(f64, f64)> {
    parse_named_value_any(line, &["size"], &["position"]).and_then(parse_point_pair)
}

fn split_key_value_once(input: &str) -> Option<(&str, &str)> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut delimiter_index = None;

    for (index, ch) in input.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        if ch == ':' || ch == '=' {
            delimiter_index = Some(index);
            break;
        }
    }
    let delimiter_index = delimiter_index?;
    let name = input[..delimiter_index].trim();
    let value = input[delimiter_index + 1..].trim();
    if name.is_empty() || value.is_empty() {
        None
    } else {
        Some((name, value))
    }
}

fn split_inline_token(token: &str) -> Option<(&str, &str)> {
    split_key_value_once(token)
}

fn parse_direction(value: &str) -> Option<Direction> {
    match value.trim().trim_matches('"').to_ascii_lowercase().as_str() {
        "right" => Some(Direction::Right),
        "left" => Some(Direction::Left),
        "up" => Some(Direction::Up),
        "down" => Some(Direction::Down),
        _ => None,
    }
}

fn parse_edge_routing(value: &str) -> Option<EdgeRouting> {
    match value.trim().trim_matches('"').to_ascii_lowercase().as_str() {
        "orthogonal" => Some(EdgeRouting::Orthogonal),
        "polyline" => Some(EdgeRouting::Polyline),
        "splines" | "spline" => Some(EdgeRouting::Splines),
        _ => None,
    }
}

fn get_or_create_node(
    graph: &ElkNodeRef,
    nodes: &mut HashMap<String, ElkNodeRef>,
    identifier: &str,
    parent_identifier: Option<&str>,
) -> ElkNodeRef {
    let storage_key = node_storage_key(identifier, parent_identifier);
    if let Some(node) = nodes.get(storage_key.as_str()) {
        return node.clone();
    }

    let parent = if let Some(parent_identifier) = parent_identifier {
        if parent_identifier == identifier {
            graph.clone()
        } else if let Some(existing_parent) =
            find_node_by_identifier_reference(nodes, parent_identifier, None)
        {
            existing_parent
        } else {
            get_or_create_node(graph, nodes, parent_identifier, None)
        }
    } else {
        graph.clone()
    };

    let node = ElkGraphUtil::create_node(Some(parent));
    set_node_dimensions(&node, DEFAULT_NODE_WIDTH, DEFAULT_NODE_HEIGHT);
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .set_identifier(Some(identifier.to_string()));
    nodes.insert(storage_key, node.clone());
    node
}

fn get_or_create_port(
    graph: &ElkNodeRef,
    nodes: &mut HashMap<String, ElkNodeRef>,
    ports: &mut HashMap<String, ElkPortRef>,
    identifier: &str,
    parent_node_id: Option<&str>,
) -> ElkPortRef {
    let storage_key = port_storage_key(identifier, parent_node_id);
    if let Some(port) = ports.get(storage_key.as_str()) {
        return port.clone();
    }

    let parent = if let Some(parent_node_id) = parent_node_id {
        get_or_create_node(graph, nodes, parent_node_id, None)
    } else {
        graph.clone()
    };
    let port = ElkGraphUtil::create_port(Some(parent));
    set_port_dimensions(&port, DEFAULT_PORT_WIDTH, DEFAULT_PORT_HEIGHT);
    port.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .set_identifier(Some(identifier.to_string()));
    ports.insert(storage_key, port.clone());
    port
}

fn set_node_dimensions(node: &ElkNodeRef, width: f64, height: f64) {
    node.borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(width, height);
}

fn set_port_dimensions(port: &ElkPortRef, width: f64, height: f64) {
    port.borrow_mut()
        .connectable()
        .shape()
        .set_dimensions(width, height);
}

fn set_node_property<T: Clone + Send + Sync + 'static>(
    node: &ElkNodeRef,
    property: &Property<T>,
    value: T,
) {
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn set_port_property<T: Clone + Send + Sync + 'static>(
    port: &ElkPortRef,
    property: &Property<T>,
    value: T,
) {
    port.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn set_port_side_property(port: &ElkPortRef, side: PortSide) {
    set_port_property(port, LayeredOptions::PORT_SIDE, side);
    set_port_property(port, CoreOptions::PORT_SIDE, side);
}

fn set_label_property<T: Clone + Send + Sync + 'static>(
    label: &org_eclipse_elk_graph::org::eclipse::elk::graph::ElkLabelRef,
    property: &Property<T>,
    value: T,
) {
    label
        .borrow_mut()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(property, Some(value));
}

fn infer_missing_port_sides(graph: &ElkNodeRef) {
    let mut queue: VecDeque<ElkNodeRef> = VecDeque::new();
    queue.push_back(graph.clone());

    while let Some(node) = queue.pop_front() {
        let ports: Vec<_> = node.borrow_mut().ports().iter().cloned().collect();
        for port in ports {
            let side = port
                .borrow_mut()
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::PORT_SIDE)
                .or_else(|| {
                    port.borrow_mut()
                        .connectable()
                        .shape()
                        .graph_element()
                        .properties_mut()
                        .get_property(LayeredOptions::PORT_SIDE)
                });

            if side.is_some_and(|value| value != PortSide::Undefined) {
                continue;
            }

            let inferred_side = infer_port_side_from_geometry(&port);
            set_port_side_property(&port, inferred_side);
        }

        let children: Vec<_> = node.borrow_mut().children().iter().cloned().collect();
        queue.extend(children);
    }
}

fn infer_port_side_from_geometry(port: &ElkPortRef) -> PortSide {
    let (port_x, port_y, port_w, port_h, parent) = {
        let mut port_mut = port.borrow_mut();
        let shape = port_mut.connectable().shape();
        (
            shape.x(),
            shape.y(),
            shape.width(),
            shape.height(),
            port_mut.parent(),
        )
    };

    let Some(parent) = parent else {
        return PortSide::East;
    };

    let (node_w, node_h) = {
        let mut node_mut = parent.borrow_mut();
        let shape = node_mut.connectable().shape();
        (shape.width(), shape.height())
    };

    if !port_x.is_finite()
        || !port_y.is_finite()
        || !port_w.is_finite()
        || !port_h.is_finite()
        || !node_w.is_finite()
        || !node_h.is_finite()
    {
        return PortSide::East;
    }

    let west_distance = port_x.abs();
    let east_distance = (node_w - (port_x + port_w)).abs();
    let north_distance = port_y.abs();
    let south_distance = (node_h - (port_y + port_h)).abs();

    let mut best = (east_distance, PortSide::East);
    if west_distance < best.0 {
        best = (west_distance, PortSide::West);
    }
    if north_distance < best.0 {
        best = (north_distance, PortSide::North);
    }
    if south_distance < best.0 {
        best = (south_distance, PortSide::South);
    }
    best.1
}
