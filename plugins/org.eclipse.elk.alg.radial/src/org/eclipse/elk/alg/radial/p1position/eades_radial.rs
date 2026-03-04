use std::collections::HashMap;
use std::f64::consts::PI;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::optimization::IEvaluation;
use crate::org::eclipse::elk::alg::radial::options::RadialOptions;
use crate::org::eclipse::elk::alg::radial::p1position::wedge::IAnnulusWedgeCriteria;
use crate::org::eclipse::elk::alg::radial::radial_layout_phases::RadialLayoutPhases;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;
use crate::org::eclipse::elk::alg::radial::sorting::IRadialSorter;

const CIRCLE_DEGREES: i32 = 360;
const DEGREE_TO_RAD: f64 = PI / 180.0;

pub struct EadesRadial;

/// SoA for radial layout — pre-extracts node sizes to avoid repeated borrows in position_nodes.
struct RadialSoA {
    nodes: Vec<ElkNodeRef>,
    /// node_key → index mapping
    key_to_idx: HashMap<usize, usize>,
    /// Half-width per node (width / 2.0)
    half_w: Vec<f64>,
    /// Half-height per node (height / 2.0)
    half_h: Vec<f64>,
    /// Leaf count per node
    leaf_count: Vec<f64>,
}

impl RadialSoA {
    /// Build SoA from tree caches — one borrow per node to extract sizes.
    fn build(
        root: &ElkNodeRef,
        successor_cache: &HashMap<usize, Vec<ElkNodeRef>>,
        leaf_cache: &HashMap<usize, f64>,
    ) -> Self {
        // BFS to discover all nodes
        let mut nodes: Vec<ElkNodeRef> = Vec::new();
        let mut key_to_idx: HashMap<usize, usize> = HashMap::new();
        let mut queue: std::collections::VecDeque<ElkNodeRef> = std::collections::VecDeque::new();

        let root_key = Rc::as_ptr(root) as usize;
        key_to_idx.insert(root_key, 0);
        nodes.push(root.clone());
        queue.push_back(root.clone());

        while let Some(node) = queue.pop_front() {
            let nk = Rc::as_ptr(&node) as usize;
            if let Some(succs) = successor_cache.get(&nk) {
                for child in succs {
                    let ck = Rc::as_ptr(child) as usize;
                    if let std::collections::hash_map::Entry::Vacant(e) = key_to_idx.entry(ck) {
                        e.insert(nodes.len());
                        nodes.push(child.clone());
                        queue.push_back(child.clone());
                    }
                }
            }
        }

        let n = nodes.len();
        let mut half_w = vec![0.0; n];
        let mut half_h = vec![0.0; n];
        let mut leaf_count = vec![1.0; n];

        for (i, node) in nodes.iter().enumerate() {
            // Single borrow to extract sizes
            let mut node_mut = node.borrow_mut();
            let shape = node_mut.connectable().shape();
            half_w[i] = shape.width() / 2.0;
            half_h[i] = shape.height() / 2.0;
            drop(node_mut);

            let nk = Rc::as_ptr(node) as usize;
            if let Some(&lc) = leaf_cache.get(&nk) {
                leaf_count[i] = lc;
            }
        }

        RadialSoA {
            nodes,
            key_to_idx,
            half_w,
            half_h,
            leaf_count,
        }
    }

    /// Set position for a node using pre-extracted sizes (single borrow for set_location).
    #[inline]
    fn center_node(&self, idx: usize, x_pos: f64, y_pos: f64) {
        let mut node_mut = self.nodes[idx].borrow_mut();
        let shape = node_mut.connectable().shape();
        shape.set_location(x_pos - self.half_w[idx], y_pos - self.half_h[idx]);
    }
}

impl EadesRadial {
    pub fn new() -> Self {
        EadesRadial
    }

    fn translate(
        root: &ElkNodeRef,
        radius: f64,
        sorter: &mut Option<Box<dyn IRadialSorter>>,
        _annulus: &dyn IAnnulusWedgeCriteria,
        optimizer: Option<&dyn IEvaluation>,
    ) {
        // Pre-compute successor and leaf count caches (one traversal for the entire tree)
        let (successor_cache, leaf_cache) = RadialUtil::build_tree_caches(root);

        // Build SoA for size data
        let soa = RadialSoA::build(root, &successor_cache, &leaf_cache);

        let mut optimal_offset = 0.0;
        let mut optimal_value = f64::MAX;

        if let Some(optimizer) = optimizer {
            for i in 0..CIRCLE_DEGREES {
                let offset = f64::from(i) * DEGREE_TO_RAD;
                Self::position_nodes_soa(
                    &soa,
                    0,
                    root,
                    radius,
                    sorter,
                    &successor_cache,
                    0.0,
                    0.0,
                    2.0 * PI,
                    offset,
                );
                let translated_value = optimizer.evaluate(root);
                if translated_value < optimal_value {
                    optimal_offset = offset;
                    optimal_value = translated_value;
                }
            }
        }
        Self::position_nodes_soa(
            &soa,
            0,
            root,
            radius,
            sorter,
            &successor_cache,
            0.0,
            0.0,
            2.0 * PI,
            optimal_offset,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn position_nodes_soa(
        soa: &RadialSoA,
        idx: usize,
        root: &ElkNodeRef,
        radius: f64,
        sorter: &mut Option<Box<dyn IRadialSorter>>,
        successor_cache: &HashMap<usize, Vec<ElkNodeRef>>,
        current_radius: f64,
        min_alpha: f64,
        max_alpha: f64,
        optimal_offset: f64,
    ) {
        let alpha_point = (min_alpha + max_alpha) / 2.0 + optimal_offset;

        let x_pos = current_radius * alpha_point.cos();
        let y_pos = current_radius * alpha_point.sin();

        soa.center_node(idx, x_pos, y_pos);

        let number_of_leafs = soa.leaf_count[idx];

        let ratio = if (current_radius + radius).abs() < f64::EPSILON {
            1.0
        } else {
            (current_radius / (current_radius + radius)).clamp(-1.0, 1.0)
        };
        let tau = 2.0 * ratio.acos();
        let (s, mut alpha) = if tau < max_alpha - min_alpha {
            (tau / number_of_leafs, (min_alpha + max_alpha - tau) / 2.0)
        } else {
            ((max_alpha - min_alpha) / number_of_leafs, min_alpha)
        };

        // Get successor ElkNodeRefs for sorting (sorter needs actual refs)
        let node = &soa.nodes[idx];
        let node_key = Rc::as_ptr(node) as usize;
        let mut successors = successor_cache
            .get(&node_key)
            .cloned()
            .unwrap_or_else(|| RadialUtil::get_successors(node));

        if let Some(sorter) = sorter.as_mut() {
            sorter.sort_for_parent(&mut successors, node, root, current_radius == 0.0);
        }

        for child in &successors {
            let child_key = Rc::as_ptr(child) as usize;
            let child_idx = match soa.key_to_idx.get(&child_key) {
                Some(&ci) => ci,
                None => continue,
            };
            let number_of_child_leafs = soa.leaf_count[child_idx];

            Self::position_nodes_soa(
                soa,
                child_idx,
                root,
                radius,
                sorter,
                successor_cache,
                current_radius + radius,
                alpha,
                alpha + s * number_of_child_leafs,
                optimal_offset,
            );
            alpha += s * number_of_child_leafs;
        }
    }
}

impl Default for EadesRadial {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<RadialLayoutPhases, ElkNodeRef> for EadesRadial {
    fn process(&mut self, graph: &mut ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Eades radial", 1.0);
        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "After");
        }

        let root = RadialUtil::root_from_graph(graph);
        let Some(root) = root else {
            return;
        };

        let radius = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RadialOptions::RADIUS)
        }
        .unwrap_or(0.0);

        let mut sorter = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RadialOptions::SORTER)
        }
        .unwrap_or_default()
        .create();

        let annulus = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RadialOptions::WEDGE_CRITERIA)
        }
        .unwrap_or_default()
        .create();

        let optimizer = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RadialOptions::OPTIMIZATION_CRITERIA)
        }
        .unwrap_or_default()
        .create();

        Self::translate(
            &root,
            radius,
            &mut sorter,
            annulus.as_ref(),
            optimizer.as_deref(),
        );

        if progress_monitor.is_logging_enabled() {
            progress_monitor.log_graph(graph, "After");
        }
        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &ElkNodeRef,
    ) -> Option<LayoutProcessorConfiguration<RadialLayoutPhases, ElkNodeRef>> {
        None
    }
}
