use std::sync::Arc;

use crate::org::eclipse::elk::alg::layered::graph::LNodeRef;
use crate::org::eclipse::elk::alg::layered::options::PortType;
use crate::org::eclipse::elk::alg::layered::p3order::abstract_barycenter_port_distributor::AbstractBarycenterPortDistributor;
use crate::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;
use crate::org::eclipse::elk::alg::layered::p3order::cross_min_snapshot::CrossMinSnapshot;
use crate::org::eclipse::elk::alg::layered::p3order::layer_total_port_distributor::LayerTotalPortDistributor;
use crate::org::eclipse::elk::alg::layered::p3order::node_relative_port_distributor::NodeRelativePortDistributor;

pub trait BarycenterPortDistributor: IInitializable + Send {
    fn calculate_port_ranks(&mut self, layer: &[LNodeRef], port_type: PortType);
    fn port_ranks(&self) -> Vec<f64>;
    fn set_snapshot(&mut self, _snapshot: Arc<CrossMinSnapshot>) {}
}

impl BarycenterPortDistributor for AbstractBarycenterPortDistributor {
    fn calculate_port_ranks(&mut self, layer: &[LNodeRef], port_type: PortType) {
        AbstractBarycenterPortDistributor::calculate_port_ranks(self, layer, port_type);
    }

    fn port_ranks(&self) -> Vec<f64> {
        self.port_ranks().clone()
    }

    fn set_snapshot(&mut self, snapshot: Arc<CrossMinSnapshot>) {
        self.snapshot = Some(snapshot);
    }
}

impl BarycenterPortDistributor for NodeRelativePortDistributor {
    fn calculate_port_ranks(&mut self, layer: &[LNodeRef], port_type: PortType) {
        NodeRelativePortDistributor::calculate_port_ranks(self, layer, port_type);
    }

    fn port_ranks(&self) -> Vec<f64> {
        self.port_ranks().clone()
    }

    fn set_snapshot(&mut self, snapshot: Arc<CrossMinSnapshot>) {
        self.inner.snapshot = Some(snapshot);
    }
}

impl BarycenterPortDistributor for LayerTotalPortDistributor {
    fn calculate_port_ranks(&mut self, layer: &[LNodeRef], port_type: PortType) {
        LayerTotalPortDistributor::calculate_port_ranks(self, layer, port_type);
    }

    fn port_ranks(&self) -> Vec<f64> {
        self.port_ranks().clone()
    }

    fn set_snapshot(&mut self, snapshot: Arc<CrossMinSnapshot>) {
        self.inner.snapshot = Some(snapshot);
    }
}
