use std::collections::HashMap;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::compaction::{
    AbstractRadiusExtensionCompaction, IRadialCompactor,
};
use crate::org::eclipse::elk::alg::radial::options::RadialOptions;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;
use crate::org::eclipse::elk::alg::radial::sorting::IRadialSorter;

#[derive(Default)]
pub struct AnnulusWedgeCompaction {
    base: AbstractRadiusExtensionCompaction,
    left_contour: HashMap<usize, Vec<ElkNodeRef>>,
    right_contour: HashMap<usize, Vec<ElkNodeRef>>,
    sorter: Option<Box<dyn IRadialSorter>>,
}

impl AnnulusWedgeCompaction {
    pub fn new() -> Self {
        AnnulusWedgeCompaction {
            base: AbstractRadiusExtensionCompaction::new(),
            left_contour: HashMap::new(),
            right_contour: HashMap::new(),
            sorter: None,
        }
    }

    fn contract_wedge(
        &mut self,
        root: &ElkNodeRef,
        predecessors: &[ElkNodeRef],
        radial_predecessor: &ElkNodeRef,
        radial_successor: &ElkNodeRef,
        current_radius_nodes: &[ElkNodeRef],
    ) {
        let current_radius_nodes = current_radius_nodes.to_vec();
        let mut is_overlapping = self.overlapping(
            predecessors,
            radial_predecessor,
            radial_successor,
            &current_radius_nodes,
        );
        let mut was_contracted = false;

        while !is_overlapping {
            self.base.contract_layer(root, &current_radius_nodes, true);
            was_contracted = true;
            is_overlapping = self.overlapping(
                predecessors,
                radial_predecessor,
                radial_successor,
                &current_radius_nodes,
            );
        }

        if was_contracted {
            self.base.contract_layer(root, &current_radius_nodes, false);
        }

        let mut next_level_nodes = RadialUtil::get_next_level_nodes(&current_radius_nodes);
        if !next_level_nodes.is_empty() {
            if let Some(sorter) = self.sorter.as_mut() {
                sorter.sort(&mut next_level_nodes);
            }
            self.contract_wedge(
                root,
                &current_radius_nodes,
                radial_predecessor,
                radial_successor,
                &next_level_nodes,
            );
        }
    }

    fn overlapping(
        &mut self,
        predecessors: &[ElkNodeRef],
        left_parent: &ElkNodeRef,
        right_parent: &ElkNodeRef,
        layer_nodes: &[ElkNodeRef],
    ) -> bool {
        let mut layer_nodes = layer_nodes.to_vec();
        if let Some(sorter) = self.sorter.as_mut() {
            sorter.sort(&mut layer_nodes);
        }

        let Some(first_node) = layer_nodes.first() else {
            return false;
        };
        if self.contour_overlap(left_parent, first_node, false) {
            return true;
        }

        let Some(last_node) = layer_nodes.last() else {
            return false;
        };
        if self.contour_overlap(right_parent, last_node, true) {
            return true;
        }

        if self.base.overlap_layer(&layer_nodes) {
            return true;
        }

        for node in &layer_nodes {
            for predecessor in predecessors {
                if self.base.overlap(node, predecessor) {
                    return true;
                }
            }
        }
        false
    }

    fn contour_overlap(
        &self,
        neighbour_wedge_parent: &ElkNodeRef,
        node: &ElkNodeRef,
        left: bool,
    ) -> bool {
        let key = node_key(neighbour_wedge_parent);
        let contour = if left {
            self.left_contour.get(&key)
        } else {
            self.right_contour.get(&key)
        };
        if let Some(contour) = contour {
            for contour_node in contour {
                if self.base.overlap(node, contour_node) {
                    return true;
                }
            }
        }
        false
    }

    fn construct_contour(&mut self, nodes: &[ElkNodeRef]) {
        for node in nodes {
            let key = node_key(node);
            self.left_contour.entry(key).or_default().push(node.clone());
            self.right_contour
                .entry(key)
                .or_default()
                .push(node.clone());

            let mut successors = RadialUtil::get_successors(node);
            if !successors.is_empty() {
                if let Some(sorter) = self.sorter.as_mut() {
                    sorter.sort(&mut successors);
                }
                self.left_contour
                    .entry(key)
                    .or_default()
                    .push(successors[0].clone());
                self.right_contour
                    .entry(key)
                    .or_default()
                    .push(successors[successors.len() - 1].clone());

                while !RadialUtil::get_next_level_nodes(&successors).is_empty() {
                    successors = RadialUtil::get_next_level_nodes(&successors);
                    if let Some(sorter) = self.sorter.as_mut() {
                        sorter.sort(&mut successors);
                    }
                    self.left_contour
                        .entry(key)
                        .or_default()
                        .push(successors[0].clone());
                    self.right_contour
                        .entry(key)
                        .or_default()
                        .push(successors[successors.len() - 1].clone());
                }
            }
        }
    }
}

impl IRadialCompactor for AnnulusWedgeCompaction {
    fn compact(&mut self, graph: &ElkNodeRef) {
        let root = RadialUtil::root_from_graph(graph);
        let Some(root) = root else {
            return;
        };

        self.sorter = {
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

        let step_size = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(RadialOptions::COMPACTION_STEP_SIZE)
        };
        if let Some(step_size) = step_size {
            self.base.set_compaction_step(step_size);
        }

        let spacing = {
            let mut graph_mut = graph.borrow_mut();
            graph_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .get_property(CoreOptions::SPACING_NODE_NODE)
        }
        .unwrap_or(0.0);
        self.base.set_spacing(spacing);

        let mut successors = RadialUtil::get_successors(&root);
        if let Some(sorter) = self.sorter.as_mut() {
            sorter.sort(&mut successors);
        }
        self.construct_contour(&successors);

        let root_list = vec![root.clone()];
        for _ in 0..2 {
            for i in 0..successors.len() {
                let wedge_parent = successors[i].clone();
                let right_parent = if i < successors.len() - 1 {
                    successors[i + 1].clone()
                } else {
                    successors[0].clone()
                };
                let left_parent = if i == 0 {
                    successors[successors.len() - 1].clone()
                } else {
                    successors[i - 1].clone()
                };
                let current_radius_nodes = vec![wedge_parent.clone()];
                self.contract_wedge(
                    &root,
                    &root_list,
                    &left_parent,
                    &right_parent,
                    &current_radius_nodes,
                );
            }
        }
    }
}

fn node_key(node: &ElkNodeRef) -> usize {
    Rc::as_ptr(node) as usize
}
