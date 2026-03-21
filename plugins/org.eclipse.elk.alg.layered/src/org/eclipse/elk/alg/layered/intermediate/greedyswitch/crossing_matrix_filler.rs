use crate::org::eclipse::elk::alg::layered::graph::LNodeRef;
use crate::org::eclipse::elk::alg::layered::intermediate::greedyswitch::between_layer_edge_two_node_crossings_counter::BetweenLayerEdgeTwoNodeCrossingsCounter;
use crate::org::eclipse::elk::alg::layered::intermediate::greedyswitch::switch_decider::CrossingCountSide;
use crate::org::eclipse::elk::alg::layered::p3order::CrossMinType;

pub struct CrossingMatrixFiller {
    is_crossing_matrix_filled: Vec<Vec<bool>>,
    crossing_matrix: Vec<Vec<i32>>,
    in_between_layer_crossing_counter: BetweenLayerEdgeTwoNodeCrossingsCounter,
    direction: CrossingCountSide,
    one_sided: bool,
}

impl CrossingMatrixFiller {
    pub fn new(
        greedy_switch_type: CrossMinType,
        graph: &[Vec<LNodeRef>],
        free_layer_index: usize,
        direction: CrossingCountSide,
    ) -> Self {
        let one_sided = greedy_switch_type == CrossMinType::OneSidedGreedySwitch;
        let free_layer_len = graph
            .get(free_layer_index)
            .map(|layer| layer.len())
            .unwrap_or(0);
        let is_crossing_matrix_filled = vec![vec![false; free_layer_len]; free_layer_len];
        let crossing_matrix = vec![vec![0; free_layer_len]; free_layer_len];

        let in_between_layer_crossing_counter =
            BetweenLayerEdgeTwoNodeCrossingsCounter::new(graph.to_vec(), free_layer_index);

        CrossingMatrixFiller {
            is_crossing_matrix_filled,
            crossing_matrix,
            in_between_layer_crossing_counter,
            direction,
            one_sided,
        }
    }

    pub fn crossing_matrix_entry(&mut self, upper_node: &LNodeRef, lower_node: &LNodeRef) -> i32 {
        let upper_id = node_id(upper_node);
        let lower_id = node_id(lower_node);
        if upper_id >= self.is_crossing_matrix_filled.len()
            || lower_id >= self.is_crossing_matrix_filled.len()
        {
            return 0;
        }
        if !self.is_crossing_matrix_filled[upper_id][lower_id] {
            self.fill_crossing_matrix(upper_node, lower_node);
            self.is_crossing_matrix_filled[upper_id][lower_id] = true;
            self.is_crossing_matrix_filled[lower_id][upper_id] = true;
        }
        self.crossing_matrix[upper_id][lower_id]
    }

    fn fill_crossing_matrix(&mut self, upper_node: &LNodeRef, lower_node: &LNodeRef) {
        if self.one_sided {
            match self.direction {
                CrossingCountSide::East => {
                    self.in_between_layer_crossing_counter
                        .count_eastern_edge_crossings(upper_node, lower_node);
                }
                CrossingCountSide::West => {
                    self.in_between_layer_crossing_counter
                        .count_western_edge_crossings(upper_node, lower_node);
                }
            }
        } else {
            self.in_between_layer_crossing_counter
                .count_both_side_crossings(upper_node, lower_node);
        }

        let upper_id = node_id(upper_node);
        let lower_id = node_id(lower_node);
        if upper_id >= self.crossing_matrix.len() || lower_id >= self.crossing_matrix.len() {
            return;
        }
        self.crossing_matrix[upper_id][lower_id] = self
            .in_between_layer_crossing_counter
            .upper_lower_crossings();
        self.crossing_matrix[lower_id][upper_id] = self
            .in_between_layer_crossing_counter
            .lower_upper_crossings();
    }
}

fn node_id(node: &LNodeRef) -> usize {
    node.lock_ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}
