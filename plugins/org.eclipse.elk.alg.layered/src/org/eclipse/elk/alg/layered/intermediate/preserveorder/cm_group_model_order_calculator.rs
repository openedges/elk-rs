use crate::org::eclipse::elk::alg::layered::graph::{LGraphRef, LNodeRef};
use crate::org::eclipse::elk::alg::layered::options::{
    GroupOrderStrategy, InternalProperties, LayeredOptions,
};

pub struct CMGroupModelOrderCalculator;

impl CMGroupModelOrderCalculator {
    pub fn calculate_model_order_or_group_model_order(
        element: &LNodeRef,
        other: &LNodeRef,
        parent: &LGraphRef,
        offset: i32,
    ) -> i32 {
        let enforce_group_model_order = match parent.try_lock() {
            Ok(mut graph_guard) => {
                graph_guard
                    .get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_GROUP_ORDER_STRATEGY)
                    .unwrap_or(GroupOrderStrategy::OnlyWithinGroup)
                    == GroupOrderStrategy::Enforced
            }
            Err(_) => {
                if std::env::var_os("ELK_TRACE_CROSSMIN").is_some() {
                    eprintln!("cm_group: graph lock busy, skipping group order");
                }
                false
            }
        };
        let enforced_orders = match parent.try_lock() {
            Ok(mut graph_guard) => graph_guard
                .get_property(LayeredOptions::GROUP_MODEL_ORDER_CM_ENFORCED_GROUP_ORDERS)
                .unwrap_or_default(),
            Err(_) => {
                if std::env::var_os("ELK_TRACE_CROSSMIN").is_some() {
                    eprintln!("cm_group: graph lock busy, using empty enforced orders");
                }
                Vec::new()
            }
        };

        let element_model_order = element
            .lock()
            .ok()
            .and_then(|mut node_guard| node_guard.get_property(InternalProperties::MODEL_ORDER));
        if element_model_order.is_none() {
            return -1;
        }
        let element_model_order = element_model_order.unwrap_or(0);

        if enforce_group_model_order {
            let element_group_id = element
                .lock()
                .ok()
                .and_then(|mut node_guard| {
                    node_guard
                        .get_property(LayeredOptions::GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID)
                })
                .unwrap_or(0);
            let other_group_id = other
                .lock()
                .ok()
                .and_then(|mut node_guard| {
                    node_guard
                        .get_property(LayeredOptions::GROUP_MODEL_ORDER_CROSSING_MINIMIZATION_ID)
                })
                .unwrap_or(0);
            if enforced_orders.contains(&element_group_id)
                && enforced_orders.contains(&other_group_id)
            {
                return offset * element_group_id + element_model_order;
            }
        }

        element_model_order
    }
}
