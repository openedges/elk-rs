use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{IElkProgressMonitor, Random};

use crate::org::eclipse::elk::alg::force::graph::{FBendpoint, FGraph, FParticleRef};
use crate::org::eclipse::elk::alg::force::options::{ForceOptions, InternalProperties};

const DISP_BOUND_FACTOR: f64 = 16.0;

#[derive(Debug, Default)]
pub struct AbstractForceModel {
    random: Random,
    disp_bound: f64,
}

pub trait ForceModel {
    fn base(&mut self) -> &mut AbstractForceModel;
    fn initialize_model(&mut self, graph: &mut FGraph);
    fn more_iterations(&mut self, count: i32) -> bool;
    fn calc_displacement(
        &mut self,
        graph: &FGraph,
        forcer: &FParticleRef,
        forcee: &FParticleRef,
    ) -> Option<KVector>;
    fn iteration_done(&mut self) {}

    fn layout(&mut self, graph: &mut FGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Component Layout", 1.0);
        self.base().initialize(graph);
        self.initialize_model(graph);

        let mut iterations = 0;
        while self.more_iterations(iterations) && !monitor.is_canceled() {
            self.base().iteration_done(graph);
            self.iteration_done();

            let particles = graph.particles();
            for v in &particles {
                for u in &particles {
                    if u.ptr_eq(v) {
                        continue;
                    }
                    if let Some(displacement) = self.calc_displacement(graph, u, v) {
                        let _ = v.with_particle_mut(|particle| {
                            particle.displacement().add(&displacement);
                        });
                    }
                }
            }

            let disp_bound = self.base().disp_bound;
            for v in &particles {
                let _ = v.with_particle_mut(|particle| {
                    let mut d = *particle.displacement_ref();
                    d.bound(-disp_bound, -disp_bound, disp_bound, disp_bound);
                    particle.position().add(&d);
                    particle.displacement().reset();
                });
            }

            iterations += 1;
        }
        monitor.done();
    }
}

impl AbstractForceModel {
    pub fn new() -> Self {
        AbstractForceModel::default()
    }

    pub fn random_mut(&mut self) -> &mut Random {
        &mut self.random
    }

    pub fn disp_bound(&self) -> f64 {
        self.disp_bound
    }

    pub fn initialize(&mut self, graph: &mut FGraph) {
        self.random = graph
            .get_property(InternalProperties::RANDOM)
            .unwrap_or_else(|| Random::new(1));

        graph.calc_adjacency();

        let disp_bound =
            (graph.nodes().len() as f64 * DISP_BOUND_FACTOR) + graph.edges().len() as f64;
        self.disp_bound = disp_bound.max(DISP_BOUND_FACTOR * DISP_BOUND_FACTOR);

        let interactive = graph
            .get_property(ForceOptions::INTERACTIVE)
            .unwrap_or(false);
        if !interactive {
            let pos_scale = graph.nodes().len() as f64;
            for node in graph.nodes() {
                {
                    let mut node_guard = node.lock();
                    let pos = node_guard.position();
                    pos.x = self.random.next_double() * pos_scale;
                    pos.y = self.random.next_double() * pos_scale;
                }
            }
        }

        let edges = graph.edges().clone();
        let mut new_bendpoints = Vec::new();
        for edge in edges {
            let count = {
                let mut edge_guard = edge.lock();
                edge_guard.get_property(ForceOptions::REPULSIVE_POWER).unwrap_or(0)
            };
            if count > 0 {
                for _ in 0..count {
                    let bend = FBendpoint::new(&edge);
                    new_bendpoints.push(bend);
                }
                {
                    let mut edge_guard = edge.lock();
                    edge_guard.distribute_bendpoints();
                }
            }
        }
        graph.bendpoints_mut().extend(new_bendpoints);
    }

    pub fn iteration_done(&mut self, graph: &mut FGraph) {
        for edge in graph.edges() {
            {
                let mut edge_guard = edge.lock();
                for label in edge_guard.labels_mut() {
                    {
                        let mut label_guard = label.lock();
                        label_guard.refresh_position();
                    }
                }
                edge_guard.distribute_bendpoints();
            }
        }
    }

    pub fn avoid_same_position(random: &mut Random, u: &FParticleRef, v: &FParticleRef) {
        loop {
            let (pu, pv) = {
                let pu = u.with_particle_ref(|p| *p.position_ref());
                let pv = v.with_particle_ref(|p| *p.position_ref());
                match (pu, pv) {
                    (Some(pu), Some(pv)) => (pu, pv),
                    _ => return,
                }
            };
            if pu.x != pv.x || pu.y != pv.y {
                return;
            }

            let mut tried_for_bendpoints = false;
            if let (Some(u_bend), Some(v_bend)) = (u.as_bendpoint(), v.as_bendpoint()) {
                if !tried_for_bendpoints {
                    let u_edge = u_bend.lock().edge();
                    let v_edge = v_bend.lock().edge();
                    if let (Some(u_edge), Some(v_edge)) = (u_edge, v_edge) {
                        let u_vec = {
                            let edge_guard = u_edge.lock();
                            let source = edge_guard.source_point();
                            let target = edge_guard.target_point();
                            match (source, target) {
                                (Some(source), Some(target)) => {
                                    KVector::from_points(&source, &target)
                                }
                                _ => return,
                            }
                        };
                        let mut orthogonal_u = KVector::new();
                        if u_vec.length() > 0.0 {
                            let length = 2.0;
                            orthogonal_u = KVector::with_values(
                                (u_vec.x / u_vec.length()) * length,
                                -(u_vec.y / u_vec.length()) * length,
                            );
                        }
                        let v_vec = {
                            let edge_guard = v_edge.lock();
                            let source = edge_guard.source_point();
                            let target = edge_guard.target_point();
                            match (source, target) {
                                (Some(source), Some(target)) => {
                                    KVector::from_points(&source, &target)
                                }
                                _ => return,
                            }
                        };
                        let mut orthogonal_v = KVector::new();
                        if v_vec.length() > 0.0 {
                            let length = 2.0;
                            orthogonal_v = KVector::with_values(
                                (v_vec.x / v_vec.length()) * length,
                                -(v_vec.y / v_vec.length()) * length,
                            );
                        }
                        let _ = u.with_particle_mut(|p| {
                            p.position().add(&orthogonal_u);
                            p.position().add(&orthogonal_v);
                        });
                        tried_for_bendpoints = true;
                    }
                }
            }

            if !tried_for_bendpoints {
                let _ = u.with_particle_mut(|p| p.position().wiggle(random, 1.0));
                let _ = v.with_particle_mut(|p| p.position().wiggle(random, 1.0));
            }
        }
    }
}
