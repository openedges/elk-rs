use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;

use crate::org::eclipse::elk::alg::mrtree::graph::{TEdgeRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::options::InternalProperties;

struct BendRef {
    edge: TEdgeRef,
    first_index: usize,
    second_index: usize,
}

impl BendRef {
    fn new(edge: TEdgeRef, first_index: usize, second_index: usize) -> Self {
        BendRef {
            edge,
            first_index,
            second_index,
        }
    }
}

pub struct MultiLevelEdgeNodeNodeGap {
    neighbor_one: Option<TNodeRef>,
    neighbor_two: Option<TNodeRef>,
    bend_points: Vec<BendRef>,
    direction: Direction,
    node_bendpoint_padding: f64,
    on_first_node_side: bool,
    on_last_node_side: bool,
}

impl MultiLevelEdgeNodeNodeGap {
    pub fn new(
        neighbor_one: Option<TNodeRef>,
        neighbor_two: Option<TNodeRef>,
        edge: TEdgeRef,
        first_index: usize,
        second_index: usize,
        direction: Direction,
        node_bendpoint_padding: f64,
    ) -> Self {
        let mut gap = MultiLevelEdgeNodeNodeGap {
            neighbor_one,
            neighbor_two,
            bend_points: vec![BendRef::new(edge, first_index, second_index)],
            direction,
            node_bendpoint_padding,
            on_first_node_side: false,
            on_last_node_side: false,
        };
        gap.update_bend_points();
        gap
    }

    pub fn add_bend_points(&mut self, edge: TEdgeRef, first_index: usize, second_index: usize) {
        self.bend_points
            .push(BendRef::new(edge, first_index, second_index));

        if self.direction.is_horizontal() {
            self.bend_points.sort_by(|a, b| {
                let a_pos = edge_target_pos(&a.edge);
                let b_pos = edge_target_pos(&b.edge);
                a_pos
                    .y
                    .partial_cmp(&b_pos.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            self.bend_points.sort_by(|a, b| {
                let a_pos = edge_target_pos(&a.edge);
                let b_pos = edge_target_pos(&b.edge);
                a_pos
                    .x
                    .partial_cmp(&b_pos.x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        self.update_bend_points();
    }

    pub fn neighbor_one(&self) -> Option<TNodeRef> {
        self.neighbor_one.clone()
    }

    pub fn neighbor_two(&self) -> Option<TNodeRef> {
        self.neighbor_two.clone()
    }

    pub fn is_on_first_node_side(&self) -> bool {
        self.on_first_node_side
    }

    pub fn is_on_last_node_side(&self) -> bool {
        self.on_last_node_side
    }

    fn update_bend_points(&mut self) {
        let count = self.bend_points.len();
        for (i, bend_ref) in self.bend_points.iter().enumerate() {
            let interpolation = (i + 1) as f64 / (count + 1) as f64;
            let (bend1, bend2) = match (self.neighbor_one.as_ref(), self.neighbor_two.as_ref()) {
                (None, None) => return,
                (Some(neighbor_one), None) => {
                    self.on_last_node_side = true;
                    let Some((pos, size, level_min, level_max)) = node_data(neighbor_one) else {
                        continue;
                    };
                    if self.direction == Direction::Left {
                        let bend_tmp =
                            pos.y + size.y + self.node_bendpoint_padding * (i as f64 + 1.0);
                        (
                            KVector::with_values(level_max + self.node_bendpoint_padding, bend_tmp),
                            KVector::with_values(level_min - self.node_bendpoint_padding, bend_tmp),
                        )
                    } else if self.direction == Direction::Right {
                        let bend_tmp =
                            pos.y + size.y + self.node_bendpoint_padding * (i as f64 + 1.0);
                        (
                            KVector::with_values(level_min - self.node_bendpoint_padding, bend_tmp),
                            KVector::with_values(level_max + self.node_bendpoint_padding, bend_tmp),
                        )
                    } else if self.direction == Direction::Up {
                        let bend_tmp =
                            pos.x + size.x + self.node_bendpoint_padding * (i as f64 + 1.0);
                        (
                            KVector::with_values(bend_tmp, level_max + self.node_bendpoint_padding),
                            KVector::with_values(bend_tmp, level_min - self.node_bendpoint_padding),
                        )
                    } else {
                        let bend_tmp =
                            pos.x + size.x + self.node_bendpoint_padding * (i as f64 + 1.0);
                        (
                            KVector::with_values(bend_tmp, level_min - self.node_bendpoint_padding),
                            KVector::with_values(bend_tmp, level_max + self.node_bendpoint_padding),
                        )
                    }
                }
                (Some(neighbor_one), Some(neighbor_two)) => {
                    let Some((pos_one, size_one, level_min, level_max)) = node_data(neighbor_one)
                    else {
                        continue;
                    };
                    let Some((pos_two, _size_two, _level_min, _level_max)) =
                        node_data(neighbor_two)
                    else {
                        continue;
                    };
                    if self.direction == Direction::Left {
                        let bend_tmp = pos_two.y * interpolation
                            + (pos_one.y + size_one.y) * (1.0 - interpolation);
                        (
                            KVector::with_values(level_max + self.node_bendpoint_padding, bend_tmp),
                            KVector::with_values(level_min - self.node_bendpoint_padding, bend_tmp),
                        )
                    } else if self.direction == Direction::Right {
                        let bend_tmp = pos_two.y * interpolation
                            + (pos_one.y + size_one.y) * (1.0 - interpolation);
                        (
                            KVector::with_values(level_min - self.node_bendpoint_padding, bend_tmp),
                            KVector::with_values(level_max + self.node_bendpoint_padding, bend_tmp),
                        )
                    } else if self.direction == Direction::Up {
                        let bend_tmp = pos_two.x * interpolation
                            + (pos_one.x + size_one.x) * (1.0 - interpolation);
                        (
                            KVector::with_values(bend_tmp, level_max + self.node_bendpoint_padding),
                            KVector::with_values(bend_tmp, level_min - self.node_bendpoint_padding),
                        )
                    } else {
                        let bend_tmp = pos_two.x * interpolation
                            + (pos_one.x + size_one.x) * (1.0 - interpolation);
                        (
                            KVector::with_values(bend_tmp, level_min - self.node_bendpoint_padding),
                            KVector::with_values(bend_tmp, level_max + self.node_bendpoint_padding),
                        )
                    }
                }
                (None, Some(neighbor_two)) => {
                    self.on_first_node_side = true;
                    let Some((pos, _size, level_min, level_max)) = node_data(neighbor_two) else {
                        continue;
                    };
                    if self.direction == Direction::Left {
                        let bend_tmp = pos.y - self.node_bendpoint_padding * (i as f64 + 1.0);
                        (
                            KVector::with_values(level_max + self.node_bendpoint_padding, bend_tmp),
                            KVector::with_values(level_min - self.node_bendpoint_padding, bend_tmp),
                        )
                    } else if self.direction == Direction::Right {
                        let bend_tmp = pos.y - self.node_bendpoint_padding * (i as f64 + 1.0);
                        (
                            KVector::with_values(level_min - self.node_bendpoint_padding, bend_tmp),
                            KVector::with_values(level_max + self.node_bendpoint_padding, bend_tmp),
                        )
                    } else if self.direction == Direction::Up {
                        let bend_tmp = pos.x - self.node_bendpoint_padding * (i as f64 + 1.0);
                        (
                            KVector::with_values(bend_tmp, level_max + self.node_bendpoint_padding),
                            KVector::with_values(bend_tmp, level_min - self.node_bendpoint_padding),
                        )
                    } else {
                        let bend_tmp = pos.x - self.node_bendpoint_padding * (i as f64 + 1.0);
                        (
                            KVector::with_values(bend_tmp, level_min - self.node_bendpoint_padding),
                            KVector::with_values(bend_tmp, level_max + self.node_bendpoint_padding),
                        )
                    }
                }
            };

            {
                let mut edge_guard = bend_ref.edge.lock();
                let bend_points = edge_guard.bend_points();
                if bend_ref.first_index < bend_points.len() {
                    bend_points.set(bend_ref.first_index, bend1);
                }
                if bend_ref.second_index < bend_points.len() {
                    bend_points.set(bend_ref.second_index, bend2);
                }
            }
        }
    }
}

fn edge_target_pos(edge: &TEdgeRef) -> KVector {
    edge.lock().target()
        .and_then(|node| {
            node.lock_ok()
                .map(|node_guard| *node_guard.position_ref())
        })
        .unwrap_or_default()
}

fn node_data(node: &TNodeRef) -> Option<(KVector, KVector, f64, f64)> {
    let mut guard = node.lock();
    let pos = *guard.position_ref();
    let size = *guard.size_ref();
    let level_min = guard
        .get_property(InternalProperties::LEVELMIN)
        .unwrap_or(0.0);
    let level_max = guard
        .get_property(InternalProperties::LEVELMAX)
        .unwrap_or(0.0);
    Some((pos, size, level_min, level_max))
}
