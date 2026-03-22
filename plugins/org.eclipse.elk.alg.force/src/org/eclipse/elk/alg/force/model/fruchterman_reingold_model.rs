use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

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
            {
                let node_guard = node.lock();
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

    /// SoA-optimized layout: pre-extracts particle data into flat arrays,
    /// runs the O(n²) force loop with zero locks, writes back after each iteration.
    fn layout(&mut self, graph: &mut FGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Component Layout", 1.0);
        self.base.initialize(graph);
        self.initialize_model(graph);

        // Pre-extract particle data into flat SoA arrays (one lock per particle)
        let particles = graph.particles();
        let n = particles.len();
        let mut positions: Vec<KVector> = Vec::with_capacity(n);
        let mut radii: Vec<f64> = Vec::with_capacity(n);
        let mut priorities: Vec<i32> = Vec::with_capacity(n);

        for p in &particles {
            positions.push(
                p.with_particle_ref(|part| *part.position_ref())
                    .unwrap_or_default(),
            );
            radii.push(p.with_particle_ref(|part| part.radius()).unwrap_or(0.0));
            priorities.push(
                p.with_particle_mut(|part| part.get_property(ForceOptions::PRIORITY))
                    .flatten()
                    .unwrap_or(1),
            );
        }

        // Pre-compute connection matrix for all particle pairs
        let adjacency = graph.adjacency();
        let mut connections = vec![0i32; n * n];
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                connections[i * n + j] = match (&particles[i], &particles[j]) {
                    (FParticleRef::Node(n1), FParticleRef::Node(n2)) => {
                        let id1 = Some(n1.lock().id());
                        let id2 = Some(n2.lock().id());
                        match (id1, id2) {
                            (Some(id1), Some(id2))
                                if id1 < adjacency.len() && id2 < adjacency.len() =>
                            {
                                adjacency[id1][id2] + adjacency[id2][id1]
                            }
                            _ => 0,
                        }
                    }
                    (FParticleRef::Bend(b1), FParticleRef::Bend(b2)) => {
                        let edge1 = b1.lock().edge();
                        let edge2 = b2.lock().edge();
                        match (edge1, edge2) {
                            (Some(e1), Some(e2)) if Arc::ptr_eq(&e1, &e2) => {
                                let mut eg = e2.lock();
                                eg.get_property(ForceOptions::PRIORITY).unwrap_or(1)
                            }
                            _ => 0,
                        }
                    }
                    _ => 0,
                };
            }
        }

        let disp_bound = self.base.disp_bound();
        let mut displacements = vec![KVector::new(); n];
        let mut iterations = 0;

        while self.more_iterations(iterations) && !monitor.is_canceled() {
            // Write positions back to particles for iteration_done (label/bendpoint refresh)
            for (i, p) in particles.iter().enumerate() {
                let _ = p.with_particle_mut(|part| {
                    *part.position() = positions[i];
                });
            }
            self.base.iteration_done(graph);
            self.iteration_done();

            // Re-read positions after iteration_done (bendpoints may have moved)
            for (i, p) in particles.iter().enumerate() {
                if let Some(pos) = p.with_particle_ref(|part| *part.position_ref()) {
                    positions[i] = pos;
                }
            }

            // Reset displacements
            for d in &mut displacements {
                d.reset();
            }

            // O(n²) force computation — zero locks
            let random = self.base.random_mut();
            for vi in 0..n {
                for ui in 0..n {
                    if ui == vi {
                        continue;
                    }

                    // Avoid same position (wiggle in SoA arrays)
                    if positions[ui].x == positions[vi].x && positions[ui].y == positions[vi].y {
                        // For bendpoints, fall back to lock-based version
                        let u_is_bend = matches!(particles[ui], FParticleRef::Bend(_));
                        let v_is_bend = matches!(particles[vi], FParticleRef::Bend(_));
                        if u_is_bend || v_is_bend {
                            AbstractForceModel::avoid_same_position(
                                random,
                                &particles[ui],
                                &particles[vi],
                            );
                            if let Some(pos) =
                                particles[ui].with_particle_ref(|p| *p.position_ref())
                            {
                                positions[ui] = pos;
                            }
                            if let Some(pos) =
                                particles[vi].with_particle_ref(|p| *p.position_ref())
                            {
                                positions[vi] = pos;
                            }
                        } else {
                            loop {
                                positions[ui].wiggle(random, 1.0);
                                positions[vi].wiggle(random, 1.0);
                                if positions[ui].x != positions[vi].x
                                    || positions[ui].y != positions[vi].y
                                {
                                    break;
                                }
                            }
                        }
                    }

                    // Compute displacement
                    let mut displacement = KVector::from_vector(&positions[vi]);
                    displacement.sub(&positions[ui]);
                    let length = displacement.length();
                    if length == 0.0 {
                        continue;
                    }

                    let d = (length - radii[ui] - radii[vi]).max(0.0);
                    let mut force = Self::repulsive(d, self.k) * (priorities[ui] as f64);

                    let connection = connections[ui * n + vi];
                    if connection > 0 {
                        force -= Self::attractive(d, self.k) * (connection as f64);
                    }

                    displacement.scale(force * self.temperature / length);
                    displacements[vi].add(&displacement);
                }
            }

            // Apply displacements with bounds
            for vi in 0..n {
                displacements[vi].bound(-disp_bound, -disp_bound, disp_bound, disp_bound);
                positions[vi].add(&displacements[vi]);
            }

            iterations += 1;
        }

        // Final write-back of positions
        for (i, p) in particles.iter().enumerate() {
            let _ = p.with_particle_mut(|part| {
                *part.position() = positions[i];
                part.displacement().reset();
            });
        }

        monitor.done();
    }
}
