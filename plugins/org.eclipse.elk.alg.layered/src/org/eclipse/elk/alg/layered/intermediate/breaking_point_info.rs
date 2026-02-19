use std::sync::{Arc, Mutex};

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LNodeRef};

pub type BreakingPointInfoRef = Arc<Mutex<BreakingPointInfo>>;

#[derive(Clone)]
pub struct BreakingPointInfo {
    pub start: LNodeRef,
    pub end: LNodeRef,
    pub node_start_edge: LEdgeRef,
    pub start_end_edge: LEdgeRef,
    pub original_edge: LEdgeRef,
    pub start_in_layer_dummy: Option<LNodeRef>,
    pub start_in_layer_edge: Option<LEdgeRef>,
    pub end_in_layer_dummy: Option<LNodeRef>,
    pub end_in_layer_edge: Option<LEdgeRef>,
    pub prev: Option<BreakingPointInfoRef>,
    pub next: Option<BreakingPointInfoRef>,
}

impl BreakingPointInfo {
    pub fn new(
        start: LNodeRef,
        end: LNodeRef,
        node_start_edge: LEdgeRef,
        start_end_edge: LEdgeRef,
        original_edge: LEdgeRef,
    ) -> BreakingPointInfoRef {
        Arc::new(Mutex::new(Self {
            start,
            end,
            node_start_edge,
            start_end_edge,
            original_edge,
            start_in_layer_dummy: None,
            start_in_layer_edge: None,
            end_in_layer_dummy: None,
            end_in_layer_edge: None,
            prev: None,
            next: None,
        }))
    }
}
