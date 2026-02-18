use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;

use crate::org::eclipse::elk::alg::force::graph::{FGraph, FParticleRef};
use crate::org::eclipse::elk::alg::force::model::abstract_force_model::{
    AbstractForceModel, ForceModel,
};
use crate::org::eclipse::elk::alg::force::options::ForceOptions;

const ZERO_FACTOR: f64 = 100.0;

#[derive(Debug, Default)]
pub struct EadesModel {
    base: AbstractForceModel,
    max_iterations: i32,
    spring_length: f64,
    repulsion_factor: f64,
}

impl EadesModel {
    pub fn new() -> Self {
        EadesModel::default()
    }

    fn repulsive(d: f64, r: f64) -> f64 {
        if d > 0.0 {
            r / (d * d)
        } else {
            r * ZERO_FACTOR
        }
    }

    pub fn attractive(d: f64, s: f64) -> f64 {
        if d > 0.0 {
            (d / s).ln()
        } else {
            -ZERO_FACTOR
        }
    }
}

impl ForceModel for EadesModel {
    fn base(&mut self) -> &mut AbstractForceModel {
        &mut self.base
    }

    fn initialize_model(&mut self, graph: &mut FGraph) {
        self.max_iterations = graph.get_property(ForceOptions::ITERATIONS).unwrap_or(300);
        self.spring_length = graph
            .get_property(ForceOptions::SPACING_NODE_NODE)
            .unwrap_or(20.0);
        self.repulsion_factor = graph.get_property(ForceOptions::REPULSION).unwrap_or(5.0);
    }

    fn more_iterations(&mut self, count: i32) -> bool {
        count < self.max_iterations
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

        let connection = graph.get_connection(forcer, forcee);
        let force = if connection > 0 {
            -Self::attractive(d, self.spring_length) * (connection as f64)
        } else {
            let priority = forcer
                .with_particle_mut(|p| p.get_property(ForceOptions::PRIORITY))
                .flatten()
                .unwrap_or(0);
            Self::repulsive(d, self.repulsion_factor) * (priority as f64)
        };

        displacement.scale(force / length);
        Some(displacement)
    }
}
