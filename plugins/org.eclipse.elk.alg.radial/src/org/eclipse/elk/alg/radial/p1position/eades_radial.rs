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

impl EadesRadial {
    pub fn new() -> Self {
        EadesRadial
    }

    fn translate(
        root: &ElkNodeRef,
        radius: f64,
        sorter: &mut Option<Box<dyn IRadialSorter>>,
        annulus: &dyn IAnnulusWedgeCriteria,
        optimizer: Option<&dyn IEvaluation>,
    ) {
        // Pre-compute successor and leaf count caches (one traversal for the entire tree)
        let (successor_cache, leaf_cache) = RadialUtil::build_tree_caches(root);

        let mut optimal_offset = 0.0;
        let mut optimal_value = f64::MAX;

        if let Some(optimizer) = optimizer {
            for i in 0..CIRCLE_DEGREES {
                let offset = f64::from(i) * DEGREE_TO_RAD;
                Self::position_nodes(
                    root,
                    root,
                    radius,
                    sorter,
                    annulus,
                    &successor_cache,
                    &leaf_cache,
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
        Self::position_nodes(
            root,
            root,
            radius,
            sorter,
            annulus,
            &successor_cache,
            &leaf_cache,
            0.0,
            0.0,
            2.0 * PI,
            optimal_offset,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn position_nodes(
        node: &ElkNodeRef,
        root: &ElkNodeRef,
        radius: f64,
        sorter: &mut Option<Box<dyn IRadialSorter>>,
        annulus: &dyn IAnnulusWedgeCriteria,
        successor_cache: &HashMap<usize, Vec<ElkNodeRef>>,
        leaf_cache: &HashMap<usize, f64>,
        current_radius: f64,
        min_alpha: f64,
        max_alpha: f64,
        optimal_offset: f64,
    ) {
        let alpha_point = (min_alpha + max_alpha) / 2.0 + optimal_offset;

        let x_pos = current_radius * alpha_point.cos();
        let y_pos = current_radius * alpha_point.sin();

        RadialUtil::center_nodes_on_radi(node, x_pos, y_pos);

        let node_key = Rc::as_ptr(node) as usize;

        // Use cached leaf count (falls back to annulus trait for uncached nodes)
        let number_of_leafs = leaf_cache
            .get(&node_key)
            .copied()
            .unwrap_or_else(|| annulus.calculate_wedge_space(node));

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

        // Use cached successors (falls back to RadialUtil for uncached nodes)
        let mut successors = successor_cache
            .get(&node_key)
            .cloned()
            .unwrap_or_else(|| RadialUtil::get_successors(node));

        if let Some(sorter) = sorter.as_mut() {
            sorter.initialize(root);
            sorter.sort(&mut successors);
        }

        for child in successors {
            let child_key = Rc::as_ptr(&child) as usize;
            let number_of_child_leafs = leaf_cache
                .get(&child_key)
                .copied()
                .unwrap_or_else(|| annulus.calculate_wedge_space(&child));

            Self::position_nodes(
                &child,
                root,
                radius,
                sorter,
                annulus,
                successor_cache,
                leaf_cache,
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
