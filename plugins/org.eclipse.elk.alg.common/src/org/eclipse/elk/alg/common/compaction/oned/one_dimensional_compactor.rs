use std::collections::HashMap;
use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkRectangle;
use org_eclipse_elk_core::org::eclipse::elk::core::options::Direction;

use super::longest_path_compaction::LongestPathCompaction;
use super::quadratic_constraint_calculation::QuadraticConstraintCalculation;
use super::scanline_constraint_calculator::ScanlineConstraintCalculator;
use super::{
    CGraphRef, CGroup, CGroupRef, CNodeRef, DefaultSpacingsHandler, ICompactionAlgorithm,
    IConstraintCalculationAlgorithm, ILockFunction, ISpacingsHandler,
};

pub struct OneDimensionalCompactor {
    compaction_algorithm: Box<dyn ICompactionAlgorithm>,
    constraint_algorithm: Box<dyn IConstraintCalculationAlgorithm>,
    pub c_graph: CGraphRef,
    pub lock_fun: Option<Box<dyn ILockFunction>>,
    pub spacings_handler: Box<dyn ISpacingsHandler>,
    pub direction: Direction,
    finished: bool,
}

impl OneDimensionalCompactor {
    pub fn new(c_graph: CGraphRef) -> Self {
        let mut compactor = OneDimensionalCompactor {
            compaction_algorithm: Box::new(LongestPathCompaction),
            constraint_algorithm: Box::new(ScanlineConstraintCalculator),
            c_graph,
            lock_fun: None,
            spacings_handler: Box::<DefaultSpacingsHandler>::default(),
            direction: Direction::Undefined,
            finished: false,
        };

        compactor.calculate_group_offsets();

        let c_nodes = compactor.c_graph.borrow().c_nodes.clone();
        for node in c_nodes {
            if node.borrow().group().is_none() {
                CGroup::create(&compactor.c_graph, std::slice::from_ref(&node));
            }
            let hitbox = node.borrow().hitbox;
            node.borrow_mut().hitbox_pre_compaction = Some(ElkRectangle::from_other(&hitbox));
        }

        compactor
    }

    pub fn set_spacings_handler(&mut self, handler: Box<dyn ISpacingsHandler>) -> &mut Self {
        self.spacings_handler = handler;
        self
    }

    pub fn set_compaction_algorithm(
        &mut self,
        compactor: Box<dyn ICompactionAlgorithm>,
    ) -> &mut Self {
        self.compaction_algorithm = compactor;
        self
    }

    pub fn set_constraint_algorithm(
        &mut self,
        algorithm: Box<dyn IConstraintCalculationAlgorithm>,
    ) -> &mut Self {
        self.constraint_algorithm = algorithm;
        self
    }

    pub fn compact(&mut self) -> &mut Self {
        if self.finished {
            panic!(
                "The {} instance has been finished already.",
                std::any::type_name::<Self>()
            );
        }

        if self.direction == Direction::Undefined {
            self.change_direction(Direction::Left);
        }

        let c_groups = self.c_graph.borrow().c_groups.clone();
        for group in &c_groups {
            let out_degree_real = group.borrow().out_degree_real;
            group.borrow_mut().out_degree = out_degree_real;
        }

        let c_nodes = self.c_graph.borrow().c_nodes.clone();
        for node in &c_nodes {
            node.borrow_mut().start_pos = f64::NEG_INFINITY;
        }

        // Move the algorithm out temporarily to avoid aliasing self borrows.
        let algorithm = std::mem::replace(
            &mut self.compaction_algorithm,
            Box::new(LongestPathCompaction),
        );
        algorithm.compact(self);
        self.compaction_algorithm = algorithm;

        self
    }

    pub fn finish(&mut self) -> &mut Self {
        self.change_direction(Direction::Left);
        self.finished = true;
        self
    }

    pub fn change_direction(&mut self, dir: Direction) -> &mut Self {
        if self.finished {
            panic!(
                "The {} instance has been finished already.",
                std::any::type_name::<Self>()
            );
        }

        if !self.c_graph.borrow().supports(dir) {
            panic!("The direction {:?} is not supported by the CGraph instance.", dir);
        }

        if dir == self.direction {
            return self;
        }

        let old_direction = self.direction;
        self.direction = dir;

        match old_direction {
            Direction::Undefined => match dir {
                Direction::Left => self.calculate_constraints(),
                Direction::Right => {
                    self.mirror_hitboxes();
                    self.calculate_constraints();
                }
                Direction::Up => {
                    self.transpose_hitboxes();
                    self.calculate_constraints();
                }
                Direction::Down => {
                    self.transpose_hitboxes();
                    self.mirror_hitboxes();
                    self.calculate_constraints();
                }
                Direction::Undefined => {}
            },
            Direction::Left => match dir {
                Direction::Right => {
                    self.mirror_hitboxes();
                    self.reverse_constraints();
                }
                Direction::Up => {
                    self.transpose_hitboxes();
                    self.calculate_constraints();
                }
                Direction::Down => {
                    self.transpose_hitboxes();
                    self.mirror_hitboxes();
                    self.calculate_constraints();
                }
                _ => {}
            },
            Direction::Right => match dir {
                Direction::Left => {
                    self.mirror_hitboxes();
                    self.reverse_constraints();
                }
                Direction::Up => {
                    self.mirror_hitboxes();
                    self.transpose_hitboxes();
                    self.calculate_constraints();
                }
                Direction::Down => {
                    self.mirror_hitboxes();
                    self.transpose_hitboxes();
                    self.mirror_hitboxes();
                    self.calculate_constraints();
                }
                _ => {}
            },
            Direction::Up => match dir {
                Direction::Left => {
                    self.transpose_hitboxes();
                    self.calculate_constraints();
                }
                Direction::Right => {
                    self.transpose_hitboxes();
                    self.mirror_hitboxes();
                    self.calculate_constraints();
                }
                Direction::Down => {
                    self.mirror_hitboxes();
                    self.reverse_constraints();
                }
                _ => {}
            },
            Direction::Down => match dir {
                Direction::Left => {
                    self.mirror_hitboxes();
                    self.transpose_hitboxes();
                    self.calculate_constraints();
                }
                Direction::Right => {
                    self.mirror_hitboxes();
                    self.transpose_hitboxes();
                    self.mirror_hitboxes();
                    self.calculate_constraints();
                }
                Direction::Up => {
                    self.mirror_hitboxes();
                    self.reverse_constraints();
                }
                _ => {}
            },
        }

        self
    }

    pub fn is_locked_group(&self, group: &CGroupRef, direction: Direction) -> bool {
        let nodes = group.borrow().c_nodes.clone();
        for node in &nodes {
            if self.is_locked_node(node, direction) {
                return true;
            }
        }
        false
    }

    pub fn is_locked_node(&self, node: &CNodeRef, direction: Direction) -> bool {
        self.lock_fun
            .as_ref()
            .is_some_and(|lock_fun| lock_fun.is_locked(node, direction))
    }

    pub fn set_lock_function(&mut self, lock_fun: Option<Box<dyn ILockFunction>>) -> &mut Self {
        self.lock_fun = lock_fun;
        self
    }

    pub fn force_constraints_recalculation(&mut self) -> &mut Self {
        self.calculate_constraints();
        self
    }

    pub fn calculate_group_offsets(&mut self) -> &mut Self {
        let groups = self.c_graph.borrow().c_groups.clone();
        for group in &groups {
            let group_nodes = group.borrow().c_nodes.clone();
            {
                let mut group_mut = group.borrow_mut();
                group_mut.reference = None;
            }

            for node in &group_nodes {
                node.borrow_mut().c_group_offset.reset();
                let candidate_x = node.borrow().hitbox.x;
                let replace_reference = {
                    let group_ref = group.borrow().reference.clone();
                    match group_ref {
                        Some(current_reference) => candidate_x < current_reference.borrow().hitbox.x,
                        None => true,
                    }
                };
                if replace_reference {
                    group.borrow_mut().reference = Some(node.clone());
                }
            }

            let reference = group.borrow().reference.clone();
            let Some(reference) = reference else {
                continue;
            };
            let ref_x = reference.borrow().hitbox.x;
            let ref_y = reference.borrow().hitbox.y;
            for node in &group_nodes {
                let mut node_mut = node.borrow_mut();
                node_mut.c_group_offset.x = node_mut.hitbox.x - ref_x;
                node_mut.c_group_offset.y = node_mut.hitbox.y - ref_y;
            }
        }
        self
    }

    fn mirror_hitboxes(&mut self) {
        let nodes = self.c_graph.borrow().c_nodes.clone();
        for node in &nodes {
            let mut node_mut = node.borrow_mut();
            node_mut.hitbox.x = -node_mut.hitbox.x - node_mut.hitbox.width;
        }
        self.calculate_group_offsets();
    }

    fn transpose_hitboxes(&mut self) {
        let nodes = self.c_graph.borrow().c_nodes.clone();
        for node in &nodes {
            let mut node_mut = node.borrow_mut();
            let tmp_x = node_mut.hitbox.x;
            node_mut.hitbox.x = node_mut.hitbox.y;
            node_mut.hitbox.y = tmp_x;

            let tmp_w = node_mut.hitbox.width;
            node_mut.hitbox.width = node_mut.hitbox.height;
            node_mut.hitbox.height = tmp_w;

            let tmp_offset_x = node_mut.c_group_offset.x;
            node_mut.c_group_offset.x = node_mut.c_group_offset.y;
            node_mut.c_group_offset.y = tmp_offset_x;
        }
        self.calculate_group_offsets();
    }

    fn calculate_constraints(&mut self) {
        let nodes = self.c_graph.borrow().c_nodes.clone();
        for node in &nodes {
            node.borrow_mut().constraints.clear();
        }

        let constraints = if self.direction.is_horizontal() {
            self.c_graph.borrow().predefined_horizontal_constraints.clone()
        } else {
            self.c_graph.borrow().predefined_vertical_constraints.clone()
        };
        for (first, second) in &constraints {
            if matches!(self.direction, Direction::Left | Direction::Up) {
                first.borrow_mut().constraints.push(second.clone());
            } else {
                second.borrow_mut().constraints.push(first.clone());
            }
        }

        let constraint_algorithm = std::mem::replace(
            &mut self.constraint_algorithm,
            Box::new(QuadraticConstraintCalculation),
        );
        constraint_algorithm.calculate_constraints(self);
        self.constraint_algorithm = constraint_algorithm;

        self.calculate_constraints_for_cgroups();
    }

    fn calculate_constraints_for_cgroups(&mut self) {
        let groups = self.c_graph.borrow().c_groups.clone();
        for group in &groups {
            let mut group_mut = group.borrow_mut();
            group_mut.out_degree = 0;
            group_mut.out_degree_real = 0;
            group_mut.incoming_constraints.clear();
        }

        for group in &groups {
            let group_nodes = group.borrow().c_nodes.clone();
            for node in &group_nodes {
                let constraints = node.borrow().constraints.clone();
                for incoming in &constraints {
                    let incoming_group = incoming.borrow().group();
                    let Some(incoming_group) = incoming_group else {
                        continue;
                    };
                    if Rc::ptr_eq(&incoming_group, group) {
                        continue;
                    }

                    {
                        let mut group_mut = group.borrow_mut();
                        if !group_mut
                            .incoming_constraints
                            .iter()
                            .any(|candidate| Rc::ptr_eq(candidate, incoming))
                        {
                            group_mut.incoming_constraints.push(incoming.clone());
                        }
                    }

                    let mut incoming_group_mut = incoming_group.borrow_mut();
                    incoming_group_mut.out_degree += 1;
                    incoming_group_mut.out_degree_real += 1;
                }
            }
        }
    }

    fn reverse_constraints(&mut self) {
        let nodes = self.c_graph.borrow().c_nodes.clone();
        let mut incoming_map: HashMap<usize, Vec<CNodeRef>> = HashMap::new();

        for node in &nodes {
            incoming_map.insert(node_key(node), Vec::new());
        }

        for node in &nodes {
            node.borrow_mut().start_pos = f64::NEG_INFINITY;
            let constraints = node.borrow().constraints.clone();
            for incoming in constraints {
                if let Some(values) = incoming_map.get_mut(&node_key(&incoming)) {
                    values.push(node.clone());
                }
            }
        }

        for node in &nodes {
            let key = node_key(node);
            node.borrow_mut().constraints = incoming_map.remove(&key).unwrap_or_default();
        }

        self.calculate_constraints_for_cgroups();
    }
}

fn node_key(node: &CNodeRef) -> usize {
    Rc::as_ptr(node) as usize
}
