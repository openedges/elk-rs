use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;

use crate::org::eclipse::elk::alg::force::graph::{FGraph, FParticleRef};
use crate::org::eclipse::elk::alg::force::model::abstract_force_model::{
    AbstractForceModel, ForceModel,
};
use crate::org::eclipse::elk::alg::force::options::ForceOptions;

const SPACING_FACTOR: f64 = 0.01;
const ZERO_FACTOR: f64 = 100.0;

#[derive(Debug, Default)]
pub struct FruchtermanReingoldModel {
    base: AbstractForceModel,
    temperature: f64,
    threshold: f64,
    k: f64,
}

impl FruchtermanReingoldModel {
    pub fn new() -> Self {
        FruchtermanReingoldModel::default()
    }

    fn repulsive(d: f64, k: f64) -> f64 {
        if d > 0.0 {
            k * k / d
        } else {
            k * k * ZERO_FACTOR
        }
    }

    pub fn attractive(d: f64, k: f64) -> f64 {
        d * d / k
    }
}

impl ForceModel for FruchtermanReingoldModel {
    fn base(&mut self) -> &mut AbstractForceModel {
        &mut self.base
    }

    fn initialize_model(&mut self, graph: &mut FGraph) {
        self.temperature = graph
            .get_property(ForceOptions::TEMPERATURE)
            .unwrap_or(0.001);
        let iterations = graph.get_property(ForceOptions::ITERATIONS).unwrap_or(300);
        if iterations > 0 {
            self.threshold = self.temperature / (iterations as f64);
        } else {
            self.threshold = self.temperature;
        }

        let n = graph.nodes().len() as f64;
        let mut total_width = 0.0;
        let mut total_height = 0.0;
        for node in graph.nodes() {
            if let Ok(node_guard) = node.lock() {
                total_width += node_guard.size_ref().x;
                total_height += node_guard.size_ref().y;
            }
        }
        let area = total_width * total_height;
        let spacing = graph
            .get_property(ForceOptions::SPACING_NODE_NODE)
            .unwrap_or(80.0);
        let c = spacing * SPACING_FACTOR;
        if n > 0.0 {
            self.k = (area / (2.0 * n)).sqrt() * c;
        } else {
            self.k = 0.0;
        }
    }

    fn more_iterations(&mut self, _count: i32) -> bool {
        self.temperature > 0.0
    }

    fn calc_displacement(
        &mut self,
        graph: &FGraph,
        forcer: &FParticleRef,
        forcee: &FParticleRef,
    ) -> Option<KVector> {
        AbstractForceModel::avoid_same_position(self.base.random_mut(), forcer, forcee);

        let displacement = forcee.with_particle_ref(|p| *p.position_ref());
        let forcer_pos = forcer.with_particle_ref(|p| *p.position_ref());
        let (mut displacement, forcer_pos) = match (displacement, forcer_pos) {
            (Some(displacement), Some(forcer_pos)) => {
                (KVector::from_vector(&displacement), forcer_pos)
            }
            _ => return None,
        };
        displacement.sub(&forcer_pos);
        let length = displacement.length();
        if length == 0.0 {
            return None;
        }
        let forcer_radius = forcer.with_particle_ref(|p| p.radius()).unwrap_or(0.0);
        let forcee_radius = forcee.with_particle_ref(|p| p.radius()).unwrap_or(0.0);
        let d = (length - forcer_radius - forcee_radius).max(0.0);

        let priority = forcer
            .with_particle_mut(|p| p.get_property(ForceOptions::PRIORITY))
            .flatten()
            .unwrap_or(1);
        let mut force = Self::repulsive(d, self.k) * (priority as f64);

        let connection = graph.get_connection(forcer, forcee);
        if connection > 0 {
            force -= Self::attractive(d, self.k) * (connection as f64);
        }

        displacement.scale(force * self.temperature / length);
        Some(displacement)
    }

    fn iteration_done(&mut self) {
        self.temperature -= self.threshold;
    }
}
