use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::force::graph::{FGraph, FParticleId};
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

        let n = graph.nodes.len() as f64;
        let mut total_width = 0.0;
        let mut total_height = 0.0;
        for &nid in &graph.nodes {
            total_width += graph.arena.node_size[nid.0].x;
            total_height += graph.arena.node_size[nid.0].y;
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
        forcer: FParticleId,
        forcee: FParticleId,
    ) -> Option<KVector> {
        // Note: avoid_same_position needs &mut graph, handled in layout() below
        let forcee_pos = *graph.arena.particle_position(forcee);
        let forcer_pos = *graph.arena.particle_position(forcer);
        let mut displacement = KVector::from_vector(&forcee_pos);
        displacement.sub(&forcer_pos);
        let length = displacement.length();
        if length == 0.0 {
            return None;
        }
        let forcer_radius = graph.arena.particle_radius(forcer);
        let forcee_radius = graph.arena.particle_radius(forcee);
        let d = (length - forcer_radius - forcee_radius).max(0.0);

        let priority = graph
            .arena
            .particle_properties(forcer)
            .get_property(ForceOptions::PRIORITY)
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

    /// SoA-optimized layout using arena directly -- zero locks.
    fn layout(&mut self, graph: &mut FGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Component Layout", 1.0);
        self.base.initialize(graph);
        self.initialize_model(graph);

        // Pre-extract particle data into flat SoA arrays
        let particles = graph.particle_ids();
        let n = particles.len();
        let mut positions: Vec<KVector> = Vec::with_capacity(n);
        let mut radii: Vec<f64> = Vec::with_capacity(n);
        let mut priorities: Vec<i32> = Vec::with_capacity(n);

        for &p in &particles {
            positions.push(*graph.arena.particle_position(p));
            radii.push(graph.arena.particle_radius(p));
            priorities.push(
                graph
                    .arena
                    .particle_properties(p)
                    .get_property(ForceOptions::PRIORITY)
                    .unwrap_or(1),
            );
        }

        // Pre-compute connection matrix for all particle pairs
        let mut connections = vec![0i32; n * n];
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                connections[i * n + j] = graph.get_connection(particles[i], particles[j]);
            }
        }

        let disp_bound = self.base.disp_bound();
        let mut displacements = vec![KVector::new(); n];
        let mut iterations = 0;

        while self.more_iterations(iterations) && !monitor.is_canceled() {
            // Write positions back to arena for iteration_done (label/bendpoint refresh)
            for (i, &p) in particles.iter().enumerate() {
                *graph.arena.particle_position_mut(p) = positions[i];
            }
            self.base.iteration_done(graph);
            self.iteration_done();

            // Re-read positions after iteration_done (bendpoints may have moved)
            for (i, &p) in particles.iter().enumerate() {
                positions[i] = *graph.arena.particle_position(p);
            }

            // Reset displacements
            for d in &mut displacements {
                d.reset();
            }

            // O(n^2) force computation -- zero locks
            let random = self.base.random_mut();
            for vi in 0..n {
                for ui in 0..n {
                    if ui == vi {
                        continue;
                    }

                    // Avoid same position (wiggle in SoA arrays)
                    if positions[ui].x == positions[vi].x && positions[ui].y == positions[vi].y {
                        let u_is_bend = matches!(particles[ui], FParticleId::Bend(_));
                        let v_is_bend = matches!(particles[vi], FParticleId::Bend(_));
                        if u_is_bend || v_is_bend {
                            // For bendpoints, use edge-aware positioning
                            // Write current positions back, call avoid_same_position, re-read
                            *graph.arena.particle_position_mut(particles[ui]) = positions[ui];
                            *graph.arena.particle_position_mut(particles[vi]) = positions[vi];
                            AbstractForceModel::avoid_same_position(
                                random,
                                graph,
                                particles[ui],
                                particles[vi],
                            );
                            positions[ui] = *graph.arena.particle_position(particles[ui]);
                            positions[vi] = *graph.arena.particle_position(particles[vi]);
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
        for (i, &p) in particles.iter().enumerate() {
            *graph.arena.particle_position_mut(p) = positions[i];
            graph.arena.particle_displacement_mut(p).reset();
        }

        monitor.done();
    }
}
