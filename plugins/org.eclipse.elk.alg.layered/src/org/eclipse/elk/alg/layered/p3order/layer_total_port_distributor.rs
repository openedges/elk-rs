use crate::org::eclipse::elk::alg::layered::graph::LNodeRef;
use crate::org::eclipse::elk::alg::layered::options::PortType;
use crate::org::eclipse::elk::alg::layered::p3order::abstract_barycenter_port_distributor::{
    AbstractBarycenterPortDistributor, PortRankStrategy,
};
use crate::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;
use crate::org::eclipse::elk::alg::layered::p3order::i_sweep_port_distributor::ISweepPortDistributor;

pub struct LayerTotalPortDistributor {
    inner: AbstractBarycenterPortDistributor,
}

impl LayerTotalPortDistributor {
    pub fn new(num_layers: usize) -> Self {
        LayerTotalPortDistributor {
            inner: AbstractBarycenterPortDistributor::new(num_layers, PortRankStrategy::LayerTotal),
        }
    }

    pub fn port_ranks(&self) -> &Vec<f64> {
        self.inner.port_ranks()
    }

    pub fn calculate_port_ranks(&mut self, layer: &[LNodeRef], port_type: PortType) {
        self.inner.calculate_port_ranks(layer, port_type);
    }
}

impl ISweepPortDistributor for LayerTotalPortDistributor {
    fn distribute_ports_while_sweeping(
        &mut self,
        order: &[Vec<LNodeRef>],
        free_layer_index: usize,
        is_forward_sweep: bool,
    ) -> bool {
        self.inner
            .distribute_ports_while_sweeping(order, free_layer_index, is_forward_sweep)
    }
}

impl IInitializable for LayerTotalPortDistributor {
    fn init_at_layer_level(&mut self, layer_index: usize, node_order: &[Vec<LNodeRef>]) {
        self.inner.init_at_layer_level(layer_index, node_order);
    }

    fn init_at_node_level(&mut self, layer_index: usize, node_index: usize, node_order: &[Vec<LNodeRef>]) {
        self.inner.init_at_node_level(layer_index, node_index, node_order);
    }

    fn init_at_port_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        port_index: usize,
        node_order: &[Vec<LNodeRef>],
    ) {
        self.inner
            .init_at_port_level(layer_index, node_index, port_index, node_order);
    }

    fn init_after_traversal(&mut self) {
        self.inner.init_after_traversal();
    }
}
