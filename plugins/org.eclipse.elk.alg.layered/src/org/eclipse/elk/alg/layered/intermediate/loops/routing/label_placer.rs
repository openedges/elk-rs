use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::labels::ILabelManager;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

use crate::org::eclipse::elk::alg::layered::graph::LMargin;
use crate::org::eclipse::elk::alg::layered::intermediate::loops::{
    Alignment, SelfHyperLoop, SelfHyperLoopRef, SelfLoopHolderRef, SelfLoopPortRef, SelfLoopType,
};
use crate::org::eclipse::elk::alg::layered::options::{LayeredOptions, SelfLoopOrderingStrategy};

pub struct LabelPlacer;

const MIN_WIDTH_EDGE_LABELS: f64 = 16.0;

impl LabelPlacer {
    pub fn prepare_labels(
        &self,
        holder: &SelfLoopHolderRef,
        label_manager: Option<&Arc<dyn ILabelManager>>,
    ) {
        let Some((node_size, loops, ordering_strategy, node_margin)) =
            holder.lock_ok().map(|holder_guard| {
                let node = holder_guard.l_node().clone();
                let (size, ordering_strategy, margin) = node
                    .lock_ok()
                    .map(|mut node_guard| {
                        (
                            *node_guard.shape().size_ref(),
                            node_guard
                                .get_property(LayeredOptions::EDGE_ROUTING_SELF_LOOP_ORDERING)
                                .unwrap_or_default(),
                            node_guard.margin().clone(),
                        )
                    })
                    .unwrap_or((
                        KVector::new(),
                        SelfLoopOrderingStrategy::Stacked,
                        crate::org::eclipse::elk::alg::layered::graph::LMargin::new(),
                    ));
                (
                    size,
                    holder_guard.sl_hyper_loops().clone(),
                    ordering_strategy,
                    margin,
                )
            })
        else {
            return;
        };

        assign_sides_and_alignments(&loops, ordering_strategy);
        if let Some(label_manager) = label_manager {
            for sl_loop in &loops {
                manage_labels(sl_loop, node_size, &node_margin, label_manager);
            }
        }
    }

    pub fn place_labels(
        &self,
        holder: &SelfLoopHolderRef,
        base_offset: f64,
        label_manager: Option<&Arc<dyn ILabelManager>>,
    ) {
        self.prepare_labels(holder, label_manager);

        let Some((l_node, node_pos, node_size, loops, ordering_strategy, mut margin)) =
            holder.lock_ok().map(|holder_guard| {
                let node = holder_guard.l_node().clone();
                let (pos, size, ordering_strategy, margin) = node
                    .lock_ok()
                    .map(|mut node_guard| {
                        (
                            *node_guard.shape().position_ref(),
                            *node_guard.shape().size_ref(),
                            node_guard
                                .get_property(LayeredOptions::EDGE_ROUTING_SELF_LOOP_ORDERING)
                                .unwrap_or_default(),
                            node_guard.margin().clone(),
                        )
                    })
                    .unwrap_or((
                        KVector::new(),
                        KVector::new(),
                        SelfLoopOrderingStrategy::Stacked,
                        crate::org::eclipse::elk::alg::layered::graph::LMargin::new(),
                    ));
                (
                    node,
                    pos,
                    size,
                    holder_guard.sl_hyper_loops().clone(),
                    ordering_strategy,
                    margin,
                )
            })
        else {
            return;
        };

        let _ = ordering_strategy;

        for sl_loop in loops {
            let Some(absolute) =
                compute_loop_label_placement(&sl_loop, node_pos, node_size, base_offset)
            else {
                continue;
            };

            if let Some(mut sl_loop_guard) = sl_loop.lock_ok() {
                if let Some(labels) = sl_loop_guard.sl_labels_mut() {
                    *labels.position_mut() = absolute;
                    labels.apply_placement(KVector::new());
                    let local_top_left =
                        KVector::with_values(absolute.x - node_pos.x, absolute.y - node_pos.y);
                    let local_bottom_right = KVector::with_values(
                        local_top_left.x + labels.size().x,
                        local_top_left.y + labels.size().y,
                    );
                    update_margins_with_point(node_size, &mut margin, &local_top_left);
                    update_margins_with_point(node_size, &mut margin, &local_bottom_right);
                }
            }
        }

        if let Some(mut node_guard) = l_node.lock_ok() {
            *node_guard.margin() = margin;
        };
    }
}

fn manage_labels(
    sl_loop: &SelfHyperLoopRef,
    node_size: KVector,
    node_margin: &LMargin,
    label_manager: &Arc<dyn ILabelManager>,
) {
    if let Some(mut sl_loop_guard) = sl_loop.lock_ok() {
        let Some(sl_labels) = sl_loop_guard.sl_labels_mut() else {
            return;
        };

        let target_width = target_width_for_label_management(sl_labels, node_size, node_margin);
        sl_labels.apply_label_management(label_manager, target_width, 2.0);
    }
}

fn target_width_for_label_management(
    sl_labels: &crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoopLabels,
    node_size: KVector,
    node_margin: &LMargin,
) -> f64 {
    let align_ref_x = sl_labels
        .alignment_reference_sl_port()
        .as_ref()
        .map(alignment_reference_x)
        .unwrap_or(0.0);

    let target_width = match sl_labels.alignment() {
        Alignment::Center => node_margin.left + node_size.x + node_margin.right,
        Alignment::Left => node_size.x - align_ref_x + node_margin.right,
        Alignment::Right => node_margin.left + align_ref_x,
        Alignment::Top => MIN_WIDTH_EDGE_LABELS,
    };

    target_width.max(MIN_WIDTH_EDGE_LABELS)
}

fn alignment_reference_x(sl_port: &SelfLoopPortRef) -> f64 {
    sl_port
        .lock_ok()
        .and_then(|port_guard| {
            port_guard.l_port().lock_ok().map(|mut port_guard| {
                port_guard.shape().position_ref().x + port_guard.anchor_ref().x
            })
        })
        .unwrap_or(0.0)
}

fn update_margins_with_point(
    node_size: KVector,
    margins: &mut crate::org::eclipse::elk::alg::layered::graph::LMargin,
    point: &KVector,
) {
    margins.left = margins.left.max(-point.x);
    margins.right = margins.right.max(point.x - node_size.x);
    margins.top = margins.top.max(-point.y);
    margins.bottom = margins.bottom.max(point.y - node_size.y);
}

fn compute_loop_label_placement(
    sl_loop: &SelfHyperLoopRef,
    node_pos: KVector,
    node_size: KVector,
    base_offset: f64,
) -> Option<KVector> {
    let (side, alignment, align_ref, label_size, slot) = {
        let mut sl_loop_guard = sl_loop.lock_ok()?;

        let (side, alignment, align_ref, label_size) = {
            let sl_labels = sl_loop_guard.sl_labels_mut()?;
            let side = sl_labels.side();
            if side == PortSide::Undefined {
                return None;
            }

            (
                side,
                sl_labels.alignment(),
                sl_labels.alignment_reference_sl_port(),
                *sl_labels.size(),
            )
        };
        let slot = sl_loop_guard.routing_slot(side).max(0);

        (side, alignment, align_ref, label_size, slot)
    };

    let local = local_position(node_size, label_size, alignment, align_ref);
    let slot_offset = base_offset + slot as f64 * (base_offset * 0.75 + 2.0);

    let mut absolute = KVector::new();
    match side {
        PortSide::North => {
            absolute.x = node_pos.x + local.x;
            absolute.y = node_pos.y - slot_offset - label_size.y;
        }
        PortSide::South => {
            absolute.x = node_pos.x + local.x;
            absolute.y = node_pos.y + node_size.y + slot_offset;
        }
        PortSide::East => {
            absolute.x = node_pos.x + node_size.x + slot_offset;
            absolute.y = node_pos.y + local.y;
        }
        PortSide::West => {
            absolute.x = node_pos.x - slot_offset - label_size.x;
            absolute.y = node_pos.y + local.y;
        }
        PortSide::Undefined => return None,
    }

    // Keep labels in graph-local positive coordinates for single-node graphs.
    absolute.x = absolute.x.max(0.0);
    absolute.y = absolute.y.max(0.0);
    Some(absolute)
}

fn assign_sides_and_alignments(
    loops: &[SelfHyperLoopRef],
    ordering_strategy: SelfLoopOrderingStrategy,
) {
    let mut northern_one_sided_loops = Vec::new();
    let mut southern_one_sided_loops = Vec::new();

    for sl_loop in loops {
        let one_sided_side = sl_loop
            .lock_ok()
            .and_then(|sl_loop_guard| {
                if sl_loop_guard.sl_labels().is_none()
                    || sl_loop_guard.self_loop_type() != Some(SelfLoopType::OneSide)
                {
                    return None;
                }
                sl_loop_guard.occupied_port_sides().iter().copied().next()
            })
            .unwrap_or(PortSide::Undefined);

        if ordering_strategy == SelfLoopOrderingStrategy::Sequenced
            && one_sided_side == PortSide::North
        {
            northern_one_sided_loops.push(sl_loop.clone());
            continue;
        }
        if ordering_strategy == SelfLoopOrderingStrategy::Sequenced
            && one_sided_side == PortSide::South
        {
            southern_one_sided_loops.push(sl_loop.clone());
            continue;
        }

        if let Some(mut sl_loop_guard) = sl_loop.lock_ok() {
            assign_side_and_alignment(&mut sl_loop_guard);
        }
    }

    if ordering_strategy == SelfLoopOrderingStrategy::Sequenced {
        assign_one_sided_sequenced(northern_one_sided_loops, PortSide::North);
        assign_one_sided_sequenced(southern_one_sided_loops, PortSide::South);
    }
}

fn assign_one_sided_sequenced(mut loops: Vec<SelfHyperLoopRef>, side: PortSide) {
    if loops.is_empty() {
        return;
    }

    if side == PortSide::North {
        loops.sort_by_key(loop_leftmost_port_id);
    } else {
        loops.sort_by_key(|sl_loop| std::cmp::Reverse(loop_leftmost_port_id(sl_loop)));
    }

    let mut left_idx = 0usize;
    let mut right_idx = loops.len().saturating_sub(1);

    while left_idx < right_idx {
        let left_loop = loops[left_idx].clone();
        let right_loop = loops[right_idx].clone();

        let left_alignment_ref = if side == PortSide::North {
            loop_rightmost_port(&left_loop)
        } else {
            loop_leftmost_port(&left_loop)
        };
        let right_alignment_ref = if side == PortSide::North {
            loop_leftmost_port(&right_loop)
        } else {
            loop_rightmost_port(&right_loop)
        };

        set_side_and_alignment(&left_loop, side, Alignment::Right, left_alignment_ref);
        set_side_and_alignment(&right_loop, side, Alignment::Left, right_alignment_ref);

        left_idx += 1;
        right_idx = right_idx.saturating_sub(1);
    }

    if left_idx == right_idx {
        set_side_and_alignment(&loops[left_idx], side, Alignment::Center, None);
    }
}

fn set_side_and_alignment(
    sl_loop: &SelfHyperLoopRef,
    side: PortSide,
    alignment: Alignment,
    alignment_reference: Option<SelfLoopPortRef>,
) {
    if let Some(mut sl_loop_guard) = sl_loop.lock_ok() {
        if let Some(sl_labels) = sl_loop_guard.sl_labels_mut() {
            sl_labels.set_side(side);
            sl_labels.set_alignment(alignment);
            sl_labels.set_alignment_reference_sl_port(alignment_reference);
        }
    }
}

fn assign_side_and_alignment(sl_loop: &mut SelfHyperLoop) {
    let self_loop_type = sl_loop.self_loop_type();
    let leftmost = sl_loop.leftmost_port();
    let rightmost = sl_loop.rightmost_port();
    let occupied = sl_loop.occupied_port_sides().clone();
    let Some(sl_labels) = sl_loop.sl_labels_mut() else {
        return;
    };
    let has_inline_labels = sl_labels.l_labels().iter().any(|label| {
        label
            .lock_ok()
            .and_then(|mut label_guard| {
                label_guard.get_property(LayeredOptions::EDGE_LABELS_INLINE)
            })
            .unwrap_or(false)
    });

    match self_loop_type {
        Some(SelfLoopType::OneSide) => {
            clear_inline_label_property(sl_labels);
            let side = occupied
                .iter()
                .copied()
                .next()
                .unwrap_or(PortSide::Undefined);
            if side == PortSide::East || side == PortSide::West {
                let top_ref = pick_topmost_port(leftmost, rightmost);
                sl_labels.set_side(side);
                sl_labels.set_alignment(Alignment::Top);
                sl_labels.set_alignment_reference_sl_port(top_ref);
            } else {
                sl_labels.set_side(side);
                sl_labels.set_alignment(Alignment::Center);
                sl_labels.set_alignment_reference_sl_port(None);
            }
        }
        Some(SelfLoopType::TwoSidesCorner) => {
            clear_inline_label_property(sl_labels);
            let left_side = leftmost
                .as_ref()
                .map(sl_port_side)
                .unwrap_or(PortSide::Undefined);
            let right_side = rightmost
                .as_ref()
                .map(sl_port_side)
                .unwrap_or(PortSide::Undefined);
            if left_side == PortSide::North {
                sl_labels.set_side(PortSide::North);
                sl_labels.set_alignment(Alignment::Left);
                sl_labels.set_alignment_reference_sl_port(leftmost);
            } else if right_side == PortSide::North {
                sl_labels.set_side(PortSide::North);
                sl_labels.set_alignment(Alignment::Right);
                sl_labels.set_alignment_reference_sl_port(rightmost);
            } else if left_side == PortSide::South {
                sl_labels.set_side(PortSide::South);
                sl_labels.set_alignment(Alignment::Right);
                sl_labels.set_alignment_reference_sl_port(leftmost);
            } else if right_side == PortSide::South {
                sl_labels.set_side(PortSide::South);
                sl_labels.set_alignment(Alignment::Left);
                sl_labels.set_alignment_reference_sl_port(rightmost);
            } else {
                sl_labels.set_side(PortSide::North);
                sl_labels.set_alignment(Alignment::Center);
                sl_labels.set_alignment_reference_sl_port(None);
            }
        }
        Some(SelfLoopType::TwoSidesOpposing) | Some(SelfLoopType::ThreeSides) => {
            if !occupied.contains(&PortSide::North) {
                sl_labels.set_side(PortSide::South);
                sl_labels.set_alignment(Alignment::Center);
                sl_labels.set_alignment_reference_sl_port(None);
            } else if !occupied.contains(&PortSide::South) {
                sl_labels.set_side(PortSide::North);
                sl_labels.set_alignment(Alignment::Center);
                sl_labels.set_alignment_reference_sl_port(None);
            } else if !occupied.contains(&PortSide::West) {
                if has_inline_labels {
                    sl_labels.set_side(PortSide::East);
                    sl_labels.set_alignment(Alignment::Center);
                    sl_labels.set_alignment_reference_sl_port(None);
                } else {
                    sl_labels.set_side(PortSide::North);
                    sl_labels.set_alignment(Alignment::Left);
                    sl_labels.set_alignment_reference_sl_port(leftmost);
                }
            } else if !occupied.contains(&PortSide::East) {
                if has_inline_labels {
                    sl_labels.set_side(PortSide::West);
                    sl_labels.set_alignment(Alignment::Center);
                    sl_labels.set_alignment_reference_sl_port(None);
                } else {
                    sl_labels.set_side(PortSide::North);
                    sl_labels.set_alignment(Alignment::Right);
                    sl_labels.set_alignment_reference_sl_port(rightmost);
                }
            } else {
                sl_labels.set_side(PortSide::North);
                sl_labels.set_alignment(Alignment::Center);
                sl_labels.set_alignment_reference_sl_port(None);
            }
        }
        Some(SelfLoopType::FourSides) | None => {
            clear_inline_label_property(sl_labels);
            let left_side = leftmost
                .as_ref()
                .map(sl_port_side)
                .unwrap_or(PortSide::Undefined);
            let right_side = rightmost
                .as_ref()
                .map(sl_port_side)
                .unwrap_or(PortSide::Undefined);
            if left_side == PortSide::North || right_side == PortSide::North {
                sl_labels.set_side(PortSide::South);
            } else {
                sl_labels.set_side(PortSide::North);
            }
            sl_labels.set_alignment(Alignment::Center);
            sl_labels.set_alignment_reference_sl_port(None);
        }
    }
}

fn clear_inline_label_property(
    sl_labels: &mut crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoopLabels,
) {
    for label in sl_labels.l_labels() {
        if let Some(mut label_guard) = label.lock_ok() {
            label_guard.set_property(LayeredOptions::EDGE_LABELS_INLINE, None);
        }
    }
}

fn loop_leftmost_port_id(sl_loop: &SelfHyperLoopRef) -> i32 {
    sl_loop
        .lock_ok()
        .and_then(|sl_loop_guard| {
            sl_loop_guard
                .leftmost_port()
                .as_ref()
                .map(SelfHyperLoop::port_id)
        })
        .unwrap_or(i32::MAX)
}

fn loop_leftmost_port(sl_loop: &SelfHyperLoopRef) -> Option<SelfLoopPortRef> {
    sl_loop
        .lock_ok()
        .and_then(|sl_loop_guard| sl_loop_guard.leftmost_port())
}

fn loop_rightmost_port(sl_loop: &SelfHyperLoopRef) -> Option<SelfLoopPortRef> {
    sl_loop
        .lock_ok()
        .and_then(|sl_loop_guard| sl_loop_guard.rightmost_port())
}

fn local_position(
    node_size: KVector,
    label_size: KVector,
    alignment: Alignment,
    alignment_reference: Option<SelfLoopPortRef>,
) -> KVector {
    let mut result = KVector::new();
    let reference = alignment_reference.and_then(|sl_port| {
        sl_port.lock_ok().and_then(|port_guard| {
            port_guard.l_port().lock_ok().map(|mut port_guard| {
                (*port_guard.shape().position_ref(), *port_guard.anchor_ref())
            })
        })
    });

    match alignment {
        Alignment::Center => {
            result.x = (node_size.x - label_size.x) / 2.0;
            result.y = (node_size.y - label_size.y) / 2.0;
        }
        Alignment::Left => {
            if let Some((pos, anchor)) = reference {
                result.x = pos.x + anchor.x;
            }
        }
        Alignment::Right => {
            if let Some((pos, anchor)) = reference {
                result.x = pos.x + anchor.x - label_size.x;
            }
        }
        Alignment::Top => {
            if let Some((pos, anchor)) = reference {
                result.y = pos.y + anchor.y;
            }
        }
    }

    result
}

fn pick_topmost_port(
    leftmost: Option<SelfLoopPortRef>,
    rightmost: Option<SelfLoopPortRef>,
) -> Option<SelfLoopPortRef> {
    match (leftmost, rightmost) {
        (Some(left), Some(right)) => {
            let left_y = sl_port_y(&left);
            let right_y = sl_port_y(&right);
            if right_y < left_y {
                Some(right)
            } else {
                Some(left)
            }
        }
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn sl_port_side(sl_port: &SelfLoopPortRef) -> PortSide {
    sl_port
        .lock_ok()
        .and_then(|port_guard| {
            port_guard
                .l_port()
                .lock_ok()
                .map(|l_port_guard| l_port_guard.side())
        })
        .unwrap_or(PortSide::Undefined)
}

fn sl_port_y(sl_port: &SelfLoopPortRef) -> f64 {
    sl_port
        .lock_ok()
        .and_then(|port_guard| {
            port_guard.l_port().lock_ok().map(|mut port_guard| {
                port_guard.shape().position_ref().y + port_guard.anchor_ref().y
            })
        })
        .unwrap_or(f64::INFINITY)
}
