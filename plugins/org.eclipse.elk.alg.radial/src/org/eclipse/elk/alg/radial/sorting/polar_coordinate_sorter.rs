use std::f64::consts::PI;

use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::options::RadialOptions;
use crate::org::eclipse::elk::alg::radial::radial_util::RadialUtil;
use crate::org::eclipse::elk::alg::radial::sorting::{IDSorter, IRadialSorter};

const DEGREE_45: f64 = 0.25 * PI;
const DEGREE_90: f64 = 0.5 * PI;
const DEGREE_135: f64 = 0.75 * PI;
const DEGREE_225: f64 = 1.25 * PI;
const DEGREE_270: f64 = 1.5 * PI;
const DEGREE_315: f64 = 1.75 * PI;

#[derive(Default)]
pub struct PolarCoordinateSorter {
    id_sorter: Option<IDSorter>,
}

impl PolarCoordinateSorter {
    fn sort_with_offset(nodes: &mut [ElkNodeRef], offset: f64) {
        nodes.sort_by(|a, b| RadialUtil::compare_polar(a, b, offset, 0.0));
    }

    fn node_arc(node: &ElkNodeRef) -> f64 {
        let mut node_mut = node.borrow_mut();
        let shape = node_mut.connectable().shape();
        let mut arc = (shape.y() + shape.height() / 2.0).atan2(shape.x() + shape.width() / 2.0);
        if arc < 0.0 {
            arc += 2.0 * PI;
        }
        arc
    }

    fn set_id_for_nodes(nodes: &mut [ElkNodeRef], id_offset: i32) -> i32 {
        let mut id = id_offset;
        let mut next_layer_id = 0;
        for node in nodes.iter() {
            let mut node_mut = node.borrow_mut();
            node_mut
                .connectable()
                .shape()
                .graph_element()
                .properties_mut()
                .set_property(RadialOptions::ORDER_ID, Some(id));
            id += 1;
            drop(node_mut);

            let mut node_successors = RadialUtil::get_successors(node);
            let arc = Self::node_arc(node);

            if !(DEGREE_45..=DEGREE_315).contains(&arc) {
                Self::sort_with_offset(&mut node_successors, PI);
            } else if arc <= DEGREE_315 && arc > DEGREE_225 {
                Self::sort_with_offset(&mut node_successors, DEGREE_270);
            } else if arc <= DEGREE_225 && arc > DEGREE_135 {
                Self::sort_with_offset(&mut node_successors, 0.0);
            } else if arc <= DEGREE_135 {
                Self::sort_with_offset(&mut node_successors, DEGREE_90);
            }

            next_layer_id = Self::set_id_for_nodes(&mut node_successors, next_layer_id);
        }
        id
    }
}

impl IRadialSorter for PolarCoordinateSorter {
    fn sort(&mut self, nodes: &mut Vec<ElkNodeRef>) {
        if nodes.is_empty() {
            return;
        }
        if self.id_sorter.is_none() {
            let root = RadialUtil::find_root_of_node(&nodes[0]);
            self.initialize(&root);
        }
        if let Some(sorter) = self.id_sorter.as_mut() {
            sorter.sort(nodes);
        }
    }

    fn initialize(&mut self, root: &ElkNodeRef) {
        self.id_sorter = Some(IDSorter);
        let mut successors = RadialUtil::get_successors(root);
        Self::sort_with_offset(&mut successors, 0.0);
        Self::set_id_for_nodes(&mut successors, 0);
    }
}
