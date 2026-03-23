use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{IElkProgressMonitor, Random};

use crate::org::eclipse::elk::alg::force::graph::{FGraph, FParticleId};
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
        forcer: FParticleId,
        forcee: FParticleId,
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

            let particles = graph.particle_ids();
            for &v in &particles {
                for &u in &particles {
                    if u == v {
                        continue;
                    }
                    if let Some(displacement) = self.calc_displacement(graph, u, v) {
                        graph.arena.particle_displacement_mut(v).add(&displacement);
                    }
                }
            }

            let disp_bound = self.base().disp_bound;
            for &v in &particles {
                let d = *graph.arena.particle_displacement(v);
                let pos = graph.arena.particle_position_mut(v);
                let mut bounded = d;
                bounded.bound(-disp_bound, -disp_bound, disp_bound, disp_bound);
                pos.add(&bounded);
                graph.arena.particle_displacement_mut(v).reset();
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
            (graph.nodes.len() as f64 * DISP_BOUND_FACTOR) + graph.edges.len() as f64;
        self.disp_bound = disp_bound.max(DISP_BOUND_FACTOR * DISP_BOUND_FACTOR);

        let interactive = graph
            .get_property(ForceOptions::INTERACTIVE)
            .unwrap_or(false);
        if !interactive {
            let pos_scale = graph.nodes.len() as f64;
            for &nid in &graph.nodes {
                let pos = &mut graph.arena.node_position[nid.0];
                pos.x = self.random.next_double() * pos_scale;
                pos.y = self.random.next_double() * pos_scale;
            }
        }

        let edge_ids: Vec<_> = graph.edges.clone();
        for eid in edge_ids {
            let count = graph.arena.edge_properties[eid.0]
                .get_property(ForceOptions::REPULSIVE_POWER)
                .unwrap_or(0);
            if count > 0 {
                for _ in 0..count {
                    let bid = graph.arena.add_bendpoint(eid);
                    graph.bendpoints.push(bid);
                }
                graph.distribute_bendpoints(eid);
            }
        }
    }

    pub fn iteration_done(&mut self, graph: &mut FGraph) {
        let edge_ids: Vec<_> = graph.edges.clone();
        for eid in edge_ids {
            let label_ids: Vec<_> = graph.arena.edge_labels[eid.0].clone();
            for lid in label_ids {
                graph.refresh_label_position(lid);
            }
            graph.distribute_bendpoints(eid);
        }
    }

    pub fn avoid_same_position(random: &mut Random, graph: &mut FGraph, u: FParticleId, v: FParticleId) {
        loop {
            let pu = *graph.arena.particle_position(u);
            let pv = *graph.arena.particle_position(v);
            if pu.x != pv.x || pu.y != pv.y {
                return;
            }

            let mut tried_for_bendpoints = false;
            if let (FParticleId::Bend(b1), FParticleId::Bend(b2)) = (u, v) {
                let u_edge = graph.arena.bend_edge[b1.0];
                let v_edge = graph.arena.bend_edge[b2.0];
                if let (Some(u_eid), Some(v_eid)) = (u_edge, v_edge) {
                    let u_vec = {
                        let source = graph.edge_source_point(u_eid);
                        let target = graph.edge_target_point(u_eid);
                        match (source, target) {
                            (Some(source), Some(target)) => KVector::from_points(&source, &target),
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
                        let source = graph.edge_source_point(v_eid);
                        let target = graph.edge_target_point(v_eid);
                        match (source, target) {
                            (Some(source), Some(target)) => KVector::from_points(&source, &target),
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
                    graph.arena.particle_position_mut(u).add(&orthogonal_u);
                    graph.arena.particle_position_mut(u).add(&orthogonal_v);
                    tried_for_bendpoints = true;
                }
            }

            if !tried_for_bendpoints {
                graph.arena.particle_position_mut(u).wiggle(random, 1.0);
                graph.arena.particle_position_mut(v).wiggle(random, 1.0);
            }
        }
    }
}
