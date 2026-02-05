use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;

use crate::org::eclipse::elk::alg::layered::graph::LNodeRef;
use crate::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;
use crate::org::eclipse::elk::alg::layered::p3order::layer_sweep_crossing_minimizer::CrossMinType;
use crate::org::eclipse::elk::alg::layered::p3order::{
    GreedyPortDistributor, LayerTotalPortDistributor, NodeRelativePortDistributor,
};

pub trait ISweepPortDistributor: IInitializable + Send {
    fn distribute_ports_while_sweeping(
        &mut self,
        order: &[Vec<LNodeRef>],
        free_layer_index: usize,
        is_forward_sweep: bool,
    ) -> bool;
}

impl dyn ISweepPortDistributor {
    pub fn create(
        cross_min_type: CrossMinType,
        random: &mut Random,
        num_layers: usize,
    ) -> Box<dyn ISweepPortDistributor> {
        if cross_min_type == CrossMinType::TwoSidedGreedySwitch {
            Box::new(GreedyPortDistributor::new())
        } else if random.next_int(2) == 0 {
            Box::new(NodeRelativePortDistributor::new(num_layers))
        } else {
            Box::new(LayerTotalPortDistributor::new(num_layers))
        }
    }
}
