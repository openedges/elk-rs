use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::intermediate::loops::{SelfHyperLoopRef, SelfLoopHolderRef};

pub struct RoutingSlotAssigner;

impl RoutingSlotAssigner {
    pub fn assign_routing_slots(&self, holder: &SelfLoopHolderRef) {
        let loops = holder
            .lock()
            .ok()
            .map(|holder_guard| holder_guard.sl_hyper_loops().clone())
            .unwrap_or_default();

        for sl_loop in &loops {
            if let Ok(mut sl_loop_guard) = sl_loop.lock() {
                sl_loop_guard.clear_routing_slots();
            }
        }

        let sides = [PortSide::North, PortSide::East, PortSide::South, PortSide::West];
        let mut routing_slot_count = vec![0; 5];

        for side in sides {
            let mut loops_on_side = loops
                .iter()
                .filter_map(|sl_loop| {
                    sl_loop
                        .lock()
                        .ok()
                        .and_then(|sl_loop_guard| {
                            if sl_loop_guard.occupied_port_sides().contains(&side) {
                                Some(sl_loop.clone())
                            } else {
                                None
                            }
                        })
                })
                .collect::<Vec<_>>();

            loops_on_side.sort_by(loop_order_key_cmp);

            for (slot, sl_loop) in loops_on_side.iter().enumerate() {
                if let Ok(mut sl_loop_guard) = sl_loop.lock() {
                    sl_loop_guard.set_routing_slot(side, slot as i32);
                }
            }

            routing_slot_count[side_index(side)] = loops_on_side.len() as i32;
        }

        if let Ok(mut holder_guard) = holder.lock() {
            *holder_guard.routing_slot_count_mut() = routing_slot_count;
        }
    }
}

fn loop_order_key_cmp(left: &SelfHyperLoopRef, right: &SelfHyperLoopRef) -> std::cmp::Ordering {
    let left_key = loop_order_key(left);
    let right_key = loop_order_key(right);
    left_key.cmp(&right_key)
}

fn loop_order_key(sl_loop: &SelfHyperLoopRef) -> (i32, i32) {
    sl_loop
        .lock()
        .ok()
        .map(|sl_loop_guard| {
            let left = sl_loop_guard
                .leftmost_port()
                .as_ref()
                .map(
                    crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id,
                )
                .unwrap_or(i32::MAX);
            let right = sl_loop_guard
                .rightmost_port()
                .as_ref()
                .map(
                    crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop::port_id,
                )
                .unwrap_or(i32::MAX);
            (left, right)
        })
        .unwrap_or((i32::MAX, i32::MAX))
}

fn side_index(side: PortSide) -> usize {
    match side {
        PortSide::Undefined => 0,
        PortSide::North => 1,
        PortSide::East => 2,
        PortSide::South => 3,
        PortSide::West => 4,
    }
}
