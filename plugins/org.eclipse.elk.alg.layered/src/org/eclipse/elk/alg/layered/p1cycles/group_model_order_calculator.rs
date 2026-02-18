use crate::org::eclipse::elk::alg::layered::graph::LNodeRef;
use crate::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayerConstraint, LayeredOptions,
};

pub struct GroupModelOrderCalculator {
    first_separate_nodes: i32,
    last_separate_nodes: i32,
}

impl GroupModelOrderCalculator {
    pub fn new() -> Self {
        GroupModelOrderCalculator {
            first_separate_nodes: 0,
            last_separate_nodes: 0,
        }
    }

    pub fn compute_constraint_model_order(&mut self, node: &LNodeRef, offset: i32) -> i32 {
        let mut model_order = self.constraint_base_model_order(node, offset * 2, offset);

        if let Some(node_model_order) = node
            .lock()
            .ok()
            .and_then(|mut node_guard| node_guard.get_property(InternalProperties::MODEL_ORDER))
        {
            model_order += node_model_order;
        }

        model_order
    }

    pub fn compute_constraint_group_model_order(
        &mut self,
        node: &LNodeRef,
        offset: i32,
        small_offset: i32,
    ) -> i32 {
        let mut model_order = self.constraint_base_model_order(node, offset * 2, offset);

        let group_id = node
            .lock()
            .ok()
            .and_then(|mut node_guard| {
                node_guard.get_property(LayeredOptions::GROUP_MODEL_ORDER_CYCLE_BREAKING_ID)
            })
            .unwrap_or(0);
        let node_model_order = node
            .lock()
            .ok()
            .and_then(|mut node_guard| node_guard.get_property(InternalProperties::MODEL_ORDER))
            .unwrap_or(0);
        model_order += group_id * small_offset + node_model_order;

        model_order
    }

    pub fn reset_internal_counters(&mut self) {
        self.first_separate_nodes = 0;
        self.last_separate_nodes = 0;
    }

    fn constraint_base_model_order(
        &mut self,
        node: &LNodeRef,
        separate_offset: i32,
        offset: i32,
    ) -> i32 {
        let constraint = node
            .lock()
            .ok()
            .and_then(|mut node_guard| {
                if node_guard
                    .shape()
                    .graph_element()
                    .properties()
                    .has_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
                {
                    node_guard.get_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
                } else {
                    None
                }
            })
            .unwrap_or(LayerConstraint::None);

        match constraint {
            LayerConstraint::FirstSeparate => {
                let value = -separate_offset + self.first_separate_nodes;
                self.first_separate_nodes += 1;
                value
            }
            LayerConstraint::First => -offset,
            LayerConstraint::Last => offset,
            LayerConstraint::LastSeparate => {
                let value = separate_offset + self.last_separate_nodes;
                self.last_separate_nodes += 1;
                value
            }
            LayerConstraint::None => 0,
        }
    }
}

impl Default for GroupModelOrderCalculator {
    fn default() -> Self {
        Self::new()
    }
}
