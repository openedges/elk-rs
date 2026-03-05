use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::compaction::AbstractRadiusExtensionCompaction;
use crate::org::eclipse::elk::alg::radial::intermediate::overlaps::IOverlapRemoval;
use crate::org::eclipse::elk::alg::radial::options::RadialOptions;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;
use crate::org::eclipse::elk::alg::radial::sorting::IRadialSorter;

#[derive(Default)]
pub struct RadiusExtensionOverlapRemoval {
    base: AbstractRadiusExtensionCompaction,
    sorter: Option<Box<dyn IRadialSorter>>,
}

impl RadiusExtensionOverlapRemoval {
    fn extend(
        &mut self,
        graph: &ElkNodeRef,
        nodes: Vec<ElkNodeRef>,
        progress_monitor: &mut dyn IElkProgressMonitor,
        successor_cache: &HashMap<usize, Vec<ElkNodeRef>>,
        root_cx: f64,
        root_cy: f64,
    ) {
        if nodes.is_empty() {
            return;
        }

        let n = nodes.len();
        // Extract SoA arrays — single borrow per node
        let mut xs = Vec::with_capacity(n);
        let mut ys = Vec::with_capacity(n);
        let mut ws = Vec::with_capacity(n);
        let mut hs = Vec::with_capacity(n);
        for node in &nodes {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            xs.push(shape.x());
            ys.push(shape.y());
            ws.push(shape.width());
            hs.push(shape.height());
        }

        let old_x0 = xs[0];
        let old_y0 = ys[0];

        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "Before removing overlaps");
        }

        // SoA overlap removal loop — zero borrows
        let spacing = self.base.get_spacing();
        let step = self.base.get_compaction_step() as f64;
        let mut iterations = 0usize;
        while AbstractRadiusExtensionCompaction::overlap_layer_soa(&xs, &ys, &ws, &hs, spacing) {
            if iterations >= 10_000 {
                break;
            }
            AbstractRadiusExtensionCompaction::contract_layer_soa(
                root_cx, root_cy, &mut xs, &mut ys, &ws, &hs, step, false,
            );
            iterations += 1;
        }

        // Write back positions — single borrow per node
        for (i, node) in nodes.iter().enumerate() {
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            shape.set_x(xs[i]);
            shape.set_y(ys[i]);
        }

        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "After removing overlaps");
        }

        // Compute moved distance from SoA arrays directly
        let moved_x = xs[0] - old_x0;
        let moved_y = ys[0] - old_y0;
        let moved_distance = (moved_x * moved_x + moved_y * moved_y).sqrt();

        // Use cached successor lookup instead of full edge traversal
        let next_level_nodes = get_next_level_cached(&nodes, successor_cache);
        if !next_level_nodes.is_empty() {
            // Batch move: extract child positions, compute new positions, write back
            let cn = next_level_nodes.len();
            let mut cxs = Vec::with_capacity(cn);
            let mut cys = Vec::with_capacity(cn);
            let mut cws = Vec::with_capacity(cn);
            let mut chs = Vec::with_capacity(cn);
            for node in &next_level_nodes {
                let mut node_mut = node.borrow_mut();
                let shape = node_mut.connectable().shape();
                cxs.push(shape.x());
                cys.push(shape.y());
                cws.push(shape.width());
                chs.push(shape.height());
            }
            for i in 0..cn {
                let cx = cxs[i] + cws[i] / 2.0;
                let cy = cys[i] + chs[i] / 2.0;
                let dx = cx - root_cx;
                let dy = cy - root_cy;
                let length = (dx * dx + dy * dy).sqrt();
                let ux = dx / length;
                let uy = dy / length;
                cxs[i] += ux * moved_distance;
                cys[i] += uy * moved_distance;
            }
            for (i, node) in next_level_nodes.iter().enumerate() {
                let mut node_mut = node.borrow_mut();
                let shape = node_mut.connectable().shape();
                shape.set_x(cxs[i]);
                shape.set_y(cys[i]);
            }
            if progress_monitor.is_logging_enabled() {
                progress_monitor.log_graph(graph, "Child movement 1");
            }
        }

        let mut next_level_nodes = next_level_nodes;
        if let Some(sorter) = self.sorter.as_mut() {
            sorter.sort(&mut next_level_nodes);
        }
        self.extend(
            graph,
            next_level_nodes,
            progress_monitor,
            successor_cache,
            root_cx,
            root_cy,
        );
    }
}

impl IOverlapRemoval for RadiusExtensionOverlapRemoval {
    fn remove_overlaps(
        &mut self,
        graph: &ElkNodeRef,
        progress_monitor: &mut dyn IElkProgressMonitor,
    ) {
        let root = RadialUtil::root_from_graph(graph);
        let Some(root) = root else {
            return;
        };
        // Batch graph property reads in a single borrow
        let (sorter_opt, spacing) = {
            let mut graph_mut = graph.borrow_mut();
            let props = graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut();
            (
                props.get_property(RadialOptions::SORTER),
                props.get_property(CoreOptions::SPACING_NODE_NODE).unwrap_or(0.0),
            )
        };
        self.sorter = sorter_opt.unwrap_or_default().create();
        self.base.set_spacing(spacing);

        // Build successor cache once for the entire tree (Part 4)
        let (successor_cache, _leaf_cache) = RadialUtil::build_tree_caches(&root);

        // Pre-compute root center once
        let (root_cx, root_cy) = {
            let mut root_mut = root.borrow_mut();
            let shape = root_mut.connectable().shape();
            (
                shape.x() + shape.width() / 2.0,
                shape.y() + shape.height() / 2.0,
            )
        };

        // Use cached successors for initial level
        let successors = successor_cache
            .get(&(Rc::as_ptr(&root) as usize))
            .cloned()
            .unwrap_or_default();
        self.extend(
            graph,
            successors,
            progress_monitor,
            &successor_cache,
            root_cx,
            root_cy,
        );
    }
}

impl ILayoutProcessor<ElkNodeRef> for RadiusExtensionOverlapRemoval {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Remove overlaps", 1.0);
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "Before");
        }
        self.remove_overlaps(graph, progress_monitor);
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "After");
        }
    }
}

/// Cached next-level lookup using pre-built successor cache.
fn get_next_level_cached(
    nodes: &[ElkNodeRef],
    cache: &HashMap<usize, Vec<ElkNodeRef>>,
) -> Vec<ElkNodeRef> {
    let mut successors = Vec::new();
    let mut seen = HashSet::new();
    for node in nodes {
        let key = Rc::as_ptr(node) as usize;
        if let Some(children) = cache.get(&key) {
            for child in children {
                let child_key = Rc::as_ptr(child) as usize;
                if seen.insert(child_key) {
                    successors.push(child.clone());
                }
            }
        }
    }
    successors
}
