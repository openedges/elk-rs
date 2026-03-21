use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, LazyLock};

use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNodeRef, LayerRef, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{
    FixedAlignment, GraphProperties, InternalProperties, LayeredOptions,
};
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

use super::aligned_layout::{BKAlignedLayout, HDirection, VDirection};
use super::aligner::BKAligner;
use super::compactor::BKCompactor;
use super::i_compactor::ICompactor;
use super::neighborhood_information::NeighborhoodInformation;
use super::util::{
    edge_key, get_blocks, node_id, node_margin_bottom, node_margin_top, node_size_y, node_to_string,
};

static HIERARCHY_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config.add_before(
        LayeredPhases::P5EdgeRouting,
        Arc::new(IntermediateProcessorStrategy::HierarchicalPortPositionProcessor),
    );
    config
});


pub struct BKNodePlacer {
    marked_edges: HashSet<usize>,
}

impl BKNodePlacer {
    pub fn new() -> Self {
        BKNodePlacer {
            marked_edges: HashSet::new(),
        }
    }
}

impl Default for BKNodePlacer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for BKNodePlacer {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Brandes & Koepf node placement", 1.0);
        let trace = ElkTrace::global().bk;
        if trace {
            eprintln!("bk: start");
        }

        self.marked_edges.clear();
        let mut ni = NeighborhoodInformation::build_for(graph);
        let layers = graph.layers().clone();
        let nodes_by_id = build_nodes_by_id(&layers, ni.node_count);

        let align = graph
            .get_property(LayeredOptions::NODE_PLACEMENT_BK_FIXED_ALIGNMENT)
            .unwrap_or(FixedAlignment::None);
        let favor_straight = graph
            .get_property(LayeredOptions::NODE_PLACEMENT_FAVOR_STRAIGHT_EDGES)
            .unwrap_or(false);
        let produce_balanced_layout =
            (align == FixedAlignment::None && !favor_straight) || align == FixedAlignment::Balanced;

        self.mark_conflicts(&layers, &ni);
        if trace {
            eprintln!("bk: conflicts marked");
        }

        let spacings = graph
            .get_property(InternalProperties::SPACINGS)
            .unwrap_or_else(|| panic!("Missing spacings configuration for BK node placement"));

        let mut layouts: Vec<BKAlignedLayout> = Vec::new();
        match align {
            FixedAlignment::LeftDown => layouts.push(BKAlignedLayout::new(
                layers.clone(),
                nodes_by_id.clone(),
                spacings.clone(),
                Some(VDirection::Down),
                Some(HDirection::Left),
            )),
            FixedAlignment::LeftUp => layouts.push(BKAlignedLayout::new(
                layers.clone(),
                nodes_by_id.clone(),
                spacings.clone(),
                Some(VDirection::Up),
                Some(HDirection::Left),
            )),
            FixedAlignment::RightDown => layouts.push(BKAlignedLayout::new(
                layers.clone(),
                nodes_by_id.clone(),
                spacings.clone(),
                Some(VDirection::Down),
                Some(HDirection::Right),
            )),
            FixedAlignment::RightUp => layouts.push(BKAlignedLayout::new(
                layers.clone(),
                nodes_by_id.clone(),
                spacings.clone(),
                Some(VDirection::Up),
                Some(HDirection::Right),
            )),
            _ => {
                // Order must match Java: RightDown, RightUp, LeftDown, LeftUp
                layouts.push(BKAlignedLayout::new(
                    layers.clone(),
                    nodes_by_id.clone(),
                    spacings.clone(),
                    Some(VDirection::Down),
                    Some(HDirection::Right),
                ));
                layouts.push(BKAlignedLayout::new(
                    layers.clone(),
                    nodes_by_id.clone(),
                    spacings.clone(),
                    Some(VDirection::Up),
                    Some(HDirection::Right),
                ));
                layouts.push(BKAlignedLayout::new(
                    layers.clone(),
                    nodes_by_id.clone(),
                    spacings.clone(),
                    Some(VDirection::Down),
                    Some(HDirection::Left),
                ));
                layouts.push(BKAlignedLayout::new(
                    layers.clone(),
                    nodes_by_id.clone(),
                    spacings.clone(),
                    Some(VDirection::Up),
                    Some(HDirection::Left),
                ));
            }
        }

        let aligner = BKAligner::new();
        for bal in layouts.iter_mut() {
            aligner.vertical_alignment(bal, &ni, &self.marked_edges);
            aligner.inside_block_shift(bal);
        }
        if trace {
            eprintln!("bk: alignment done");
        }

        let mut compactor = BKCompactor::new(graph);
        for bal in layouts.iter_mut() {
            compactor.horizontal_compaction(bal, &ni);
        }
        if trace {
            eprintln!("bk: compaction done");
        }
        if ElkTrace::global().bk_layouts {
            for bal in &layouts {
                eprintln!("bk-layout: {}", bal);
                for layer in &layers {
                    let nodes = layer
                        .lock_ok()
                        .map(|layer_guard| layer_guard.nodes().clone())
                        .unwrap_or_default();
                    for node in nodes {
                        let id = node_id(&node);
                        eprintln!(
                            "bk-layout-node: layout={} id={} y={:.3} inner={:.3} final={:.3}",
                            bal,
                            id,
                            bal.y[id].unwrap_or(0.0),
                            bal.inner_shift[id],
                            bal.y[id].unwrap_or(0.0) + bal.inner_shift[id]
                        );
                    }
                }
            }
        }

        if monitor.is_logging_enabled() {
            for bal in layouts.iter() {
                monitor.log(&format!("{} size is {}", bal, bal.layout_size()));
            }
        }

        let mut chosen_layout: Option<&BKAlignedLayout> = None;
        let mut balanced_layout: Option<BKAlignedLayout> = None;

        if produce_balanced_layout {
            let balanced = create_balanced_layout(&layouts, &layers, &nodes_by_id);
            let balanced_ok = check_order_constraint(&balanced, &layers, monitor);
            if trace {
                eprintln!(
                    "bk: balanced candidate={} size={} order_ok={}",
                    balanced,
                    balanced.layout_size(),
                    balanced_ok
                );
            }
            if balanced_ok {
                balanced_layout = Some(balanced);
            }
        }
        if trace {
            eprintln!("bk: balanced layout checked");
        }

        if let Some(balanced) = balanced_layout.as_ref() {
            chosen_layout = Some(balanced);
        }

        if chosen_layout.is_none() {
            for bal in layouts.iter() {
                let order_ok = check_order_constraint(bal, &layers, monitor);
                if trace {
                    eprintln!(
                        "bk: candidate={} size={} order_ok={}",
                        bal,
                        bal.layout_size(),
                        order_ok
                    );
                }
                if order_ok
                    && chosen_layout
                        .map(|chosen| chosen.layout_size() > bal.layout_size())
                        .unwrap_or(true)
                {
                    chosen_layout = Some(bal);
                }
            }
        }
        if trace {
            if let Some(layout) = chosen_layout {
                eprintln!("bk: layout chosen={layout} size={}", layout.layout_size());
            } else {
                eprintln!("bk: layout chosen=<none>");
            }
        }

        let chosen_layout = chosen_layout
            .unwrap_or_else(|| layouts.first().expect("At least one BK layout must exist"));

        if ElkTrace::global().bk_node_state {
            let filter = ElkTrace::global().bk_node_filter.as_deref();
            for layer in &layers {
                let nodes = layer
                    .lock_ok()
                    .map(|layer_guard| layer_guard.nodes().clone())
                    .unwrap_or_default();
                for node in nodes {
                    let node_id = node_id(&node);
                    let (name, label_opt) = node
                        .lock_ok()
                        .map(|mut node_guard| {
                            let name = node_guard.designation().to_string();
                            let label_opt = node_guard.labels().first().and_then(|label| {
                                label
                                    .lock_ok()
                                    .map(|label_guard| label_guard.text().to_string())
                            });
                            (name, label_opt)
                        })
                        .unwrap_or_else(|| ("<poisoned>".to_string(), None));

                    if let Some(filter) = &filter {
                        if !name.contains(filter)
                            && !label_opt
                                .as_deref()
                                .is_some_and(|label| label.contains(filter))
                        {
                            continue;
                        }
                    }

                    let root_id = chosen_layout.root[node_id];
                    let align_id = chosen_layout.align[node_id];
                    let sink_id = chosen_layout.sink[node_id];
                    let y_root = chosen_layout.y[root_id].unwrap_or(0.0);
                    let y_node = chosen_layout.y[node_id].unwrap_or(0.0);
                    let inner = chosen_layout.inner_shift[node_id];
                    let block_size = chosen_layout.block_size[root_id];
                    eprintln!(
                        "bk-node-state: node={} label={:?} id={} root={} align={} sink={} y_root={:.3} y_node={:.3} inner={:.3} block_size={:.3}",
                        name, label_opt, node_id, root_id, align_id, sink_id, y_root, y_node, inner, block_size
                    );
                }
            }
        }

        for layer in &layers {
            let nodes = layer
                .lock_ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let node_id = node_id(&node);
                if let Some(mut node_guard) = node.lock_ok() {
                    node_guard.shape().position().y = chosen_layout.y[node_id].unwrap_or(0.0)
                        + chosen_layout.inner_shift[node_id];
                }
            }
        }
        if trace {
            eprintln!("bk: positions applied");
        }

        if monitor.is_logging_enabled() {
            monitor.log(&format!("Chosen node placement: {}", chosen_layout));
            let blocks = get_blocks(chosen_layout);
            monitor.log(&format!("Blocks: {}", format_blocks(&blocks)));
            let classes = get_classes(chosen_layout, monitor);
            monitor.log(&format!("Classes: {}", format_blocks(&classes)));
            monitor.log(&format!("Marked edges: {}", self.marked_edges.len()));
        }

        for bal in layouts.iter_mut() {
            bal.cleanup();
        }
        ni.cleanup();
        self.marked_edges.clear();

        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        if graph
            .get_property_ref(InternalProperties::GRAPH_PROPERTIES)
            .is_some_and(|props| props.contains(&GraphProperties::ExternalPorts))
        {
            Some(LayoutProcessorConfiguration::create_from(
                &HIERARCHY_PROCESSING_ADDITIONS,
            ))
        } else {
            None
        }
    }
}

fn build_nodes_by_id(layers: &[LayerRef], node_count: usize) -> Vec<LNodeRef> {
    let mut nodes: Vec<Option<LNodeRef>> = vec![None; node_count];
    for layer in layers {
        let layer_nodes = layer
            .lock_ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();
        for node in layer_nodes {
            let id = node_id(&node);
            if id < nodes.len() {
                nodes[id] = Some(node);
            }
        }
    }
    nodes
        .into_iter()
        .map(|node| node.expect("Missing node for id"))
        .collect()
}

impl BKNodePlacer {
    fn mark_conflicts(&mut self, layers: &[LayerRef], ni: &NeighborhoodInformation) {
        const MIN_LAYERS_FOR_CONFLICTS: usize = 3;
        let trace_conflicts = ElkTrace::global().bk_conflicts;
        if layers.len() < MIN_LAYERS_FOR_CONFLICTS {
            return;
        }

        let layer_size: Vec<usize> = layers
            .iter()
            .map(|layer| {
                layer
                    .lock_ok()
                    .map(|layer_guard| layer_guard.nodes().len())
                    .unwrap_or(0)
            })
            .collect();

        for i in 1..layers.len() - 1 {
            let current_layer = layers[i + 1].clone();
            let nodes = current_layer
                .lock_ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            let mut k_0 = 0usize;
            let mut l = 0usize;

            for l_1 in 0..layer_size[i + 1] {
                let v_l_i = nodes[l_1].clone();
                if l_1 == layer_size[i + 1] - 1
                    || self.incident_to_inner_segment(&v_l_i, i + 1, i, ni)
                {
                    let mut k_1 = layer_size[i].saturating_sub(1);
                    if self.incident_to_inner_segment(&v_l_i, i + 1, i, ni) {
                        if let Some(pair) = ni
                            .left_neighbors
                            .get(node_id(&v_l_i))
                            .and_then(|list| list.first())
                        {
                            let neighbor_id = node_id(&pair.first);
                            k_1 = *ni.node_index.get(neighbor_id).unwrap_or(&0);
                        }
                    }

                    while l <= l_1 {
                        let v_l = nodes[l].clone();
                        if !self.incident_to_inner_segment(&v_l, i + 1, i, ni) {
                            if let Some(neighbors) = ni.left_neighbors.get(node_id(&v_l)) {
                                for neighbor in neighbors {
                                    let neighbor_id = node_id(&neighbor.first);
                                    let k = *ni.node_index.get(neighbor_id).unwrap_or(&0);
                                    if k < k_0 || k > k_1 {
                                        self.marked_edges.insert(edge_key(&neighbor.second));
                                        if trace_conflicts {
                                            let source = neighbor
                                                .second
                                                .lock_ok()
                                                .and_then(|edge_guard| edge_guard.source())
                                                .and_then(|port| {
                                                    port.lock_ok()
                                                        .and_then(|port_guard| port_guard.node())
                                                });
                                            let target = neighbor
                                                .second
                                                .lock_ok()
                                                .and_then(|edge_guard| edge_guard.target())
                                                .and_then(|port| {
                                                    port.lock_ok()
                                                        .and_then(|port_guard| port_guard.node())
                                                });
                                            let src_name = source
                                                .as_ref()
                                                .and_then(|node| {
                                                    node.lock_ok().map(|mut node_guard| {
                                                        node_guard.designation().to_string()
                                                    })
                                                })
                                                .unwrap_or_else(|| "<none>".to_string());
                                            let tgt_name = target
                                                .as_ref()
                                                .and_then(|node| {
                                                    node.lock_ok().map(|mut node_guard| {
                                                        node_guard.designation().to_string()
                                                    })
                                                })
                                                .unwrap_or_else(|| "<none>".to_string());
                                            let src_id =
                                                source.as_ref().map(node_id).unwrap_or(usize::MAX);
                                            let tgt_id =
                                                target.as_ref().map(node_id).unwrap_or(usize::MAX);
                                            eprintln!(
                                                "bk-conflict: mark edge {}({src_name})->{}({tgt_name}) k={} k0={} k1={}",
                                                src_id, tgt_id, k, k_0, k_1
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        l += 1;
                    }

                    k_0 = k_1;
                }
            }
        }
    }

    fn incident_to_inner_segment(
        &self,
        node: &LNodeRef,
        layer1: usize,
        layer2: usize,
        ni: &NeighborhoodInformation,
    ) -> bool {
        if node
            .lock_ok()
            .map(|node_guard| node_guard.node_type())
            .unwrap_or(NodeType::Normal)
            != NodeType::LongEdge
        {
            return false;
        }

        let incoming_edges = node
            .lock_ok()
            .map(|node_guard| node_guard.incoming_edges())
            .unwrap_or_default();
        for edge in incoming_edges {
            let source_node = edge
                .lock_ok()
                .and_then(|edge_guard| edge_guard.source())
                .and_then(|port| port.lock_ok().and_then(|port_guard| port_guard.node()));
            if let Some(source_node) = source_node {
                let is_long_edge = source_node
                    .lock_ok()
                    .map(|node_guard| node_guard.node_type() == NodeType::LongEdge)
                    .unwrap_or(false);
                if !is_long_edge {
                    continue;
                }

                let source_layer_id = source_node
                    .lock_ok()
                    .and_then(|node_guard| node_guard.layer())
                    .and_then(|layer| {
                        layer
                            .lock_ok()
                            .map(|mut layer_guard| layer_guard.graph_element().id as usize)
                    })
                    .unwrap_or(0);
                let node_layer_id = node
                    .lock_ok()
                    .and_then(|node_guard| node_guard.layer())
                    .and_then(|layer| {
                        layer
                            .lock_ok()
                            .map(|mut layer_guard| layer_guard.graph_element().id as usize)
                    })
                    .unwrap_or(0);

                if ni.layer_index.get(source_layer_id).copied().unwrap_or(0) == layer2
                    && ni.layer_index.get(node_layer_id).copied().unwrap_or(0) == layer1
                {
                    return true;
                }
            }
        }
        false
    }
}

fn create_balanced_layout(
    layouts: &[BKAlignedLayout],
    layers: &[LayerRef],
    nodes_by_id: &[LNodeRef],
) -> BKAlignedLayout {
    let spacings = layouts
        .first()
        .map(|layout| layout.spacings.clone())
        .expect("At least one BK layout required for balanced layout");
    let mut balanced =
        BKAlignedLayout::new(layers.to_vec(), nodes_by_id.to_vec(), spacings, None, None);

    let no_of_layouts = layouts.len();
    let mut width = vec![0.0; no_of_layouts];
    let mut min = vec![f64::INFINITY; no_of_layouts];
    let mut max = vec![f64::NEG_INFINITY; no_of_layouts];
    let mut min_width_layout = 0usize;

    for (i, layout) in layouts.iter().enumerate() {
        width[i] = layout.layout_size();
        if width[i] < width[min_width_layout] {
            min_width_layout = i;
        }

        for layer in layers {
            let nodes = layer
                .lock_ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                let node_id = node_id(&node);
                let node_pos = layout.y[node_id].unwrap_or(0.0) + layout.inner_shift[node_id];
                min[i] = min[i].min(node_pos);
                max[i] = max[i].max(node_pos + node_size_y(&node));
            }
        }
    }

    let mut shift = vec![0.0; no_of_layouts];
    for i in 0..no_of_layouts {
        if layouts[i].vdir == Some(VDirection::Down) {
            shift[i] = min[min_width_layout] - min[i];
        } else {
            shift[i] = max[min_width_layout] - max[i];
        }
    }

    let mut calculated = vec![0.0; no_of_layouts];
    for layer in layers {
        let nodes = layer
            .lock_ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();
        for node in nodes {
            let node_id = node_id(&node);
            for i in 0..no_of_layouts {
                calculated[i] = layouts[i].y[node_id].unwrap_or(0.0)
                    + layouts[i].inner_shift[node_id]
                    + shift[i];
            }
            calculated.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            if no_of_layouts >= 3 {
                balanced.y[node_id] = Some((calculated[1] + calculated[2]) / 2.0);
            } else if no_of_layouts > 0 {
                balanced.y[node_id] = Some(calculated[no_of_layouts / 2]);
            }
            balanced.inner_shift[node_id] = 0.0;
        }
    }

    balanced
}

fn check_order_constraint(
    bal: &BKAlignedLayout,
    layers: &[LayerRef],
    monitor: &mut dyn IElkProgressMonitor,
) -> bool {
    let mut feasible = true;

    for layer in layers {
        let nodes = layer
            .lock_ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();
        let mut pos = f64::NEG_INFINITY;
        let mut previous: Option<LNodeRef> = None;

        for node in nodes {
            let node_id = node_id(&node);
            let top =
                bal.y[node_id].unwrap_or(0.0) + bal.inner_shift[node_id] - node_margin_top(&node);
            let bottom = bal.y[node_id].unwrap_or(0.0)
                + bal.inner_shift[node_id]
                + node_size_y(&node)
                + node_margin_bottom(&node);

            if top > pos && bottom > pos {
                pos = bal.y[node_id].unwrap_or(0.0)
                    + bal.inner_shift[node_id]
                    + node_size_y(&node)
                    + node_margin_bottom(&node);
                previous = Some(node);
            } else {
                feasible = false;
                if monitor.is_logging_enabled() {
                    let prev_str = previous
                        .as_ref()
                        .map(node_to_string)
                        .unwrap_or_else(|| "<none>".to_string());
                    monitor.log(&format!(
                        "bk node placement breaks on {} which should have been after {}",
                        node_to_string(&node),
                        prev_str
                    ));
                }
                break;
            }
        }

        if !feasible {
            break;
        }
    }

    if monitor.is_logging_enabled() {
        monitor.log(&format!("{} is feasible: {}", bal, feasible));
    }

    feasible
}

fn get_classes(
    bal: &BKAlignedLayout,
    monitor: &mut dyn IElkProgressMonitor,
) -> BTreeMap<usize, Vec<LNodeRef>> {
    let mut classes: BTreeMap<usize, Vec<LNodeRef>> = BTreeMap::new();

    if bal.vdir.is_none() {
        if monitor.is_logging_enabled() {
            monitor.log("There are no classes in a balanced layout.");
        }
        return classes;
    }

    let mut roots: HashSet<usize> = bal.root.iter().copied().collect();
    for root_id in roots.drain() {
        let sink_id = bal.sink[root_id];
        classes
            .entry(sink_id)
            .or_default()
            .push(bal.nodes_by_id[root_id].clone());
    }

    classes
}

fn format_blocks(blocks: &BTreeMap<usize, Vec<LNodeRef>>) -> String {
    let mut entries = Vec::new();
    for (root, nodes) in blocks {
        let names = nodes
            .iter()
            .map(node_to_string)
            .collect::<Vec<_>>()
            .join(", ");
        entries.push(format!("{root}:[{names}]"));
    }
    format!("{{{}}}", entries.join(", "))
}
