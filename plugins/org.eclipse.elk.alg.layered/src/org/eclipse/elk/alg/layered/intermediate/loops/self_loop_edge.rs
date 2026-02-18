use std::sync::{Arc, Mutex, Weak};

use crate::org::eclipse::elk::alg::layered::graph::LEdgeRef;
use crate::org::eclipse::elk::alg::layered::intermediate::loops::{
    SelfHyperLoopRef, SelfLoopPortRef,
};

pub type SelfLoopEdgeRef = Arc<Mutex<SelfLoopEdge>>;

pub struct SelfLoopEdge {
    l_edge: LEdgeRef,
    sl_hyper_loop: Option<
        Weak<Mutex<crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfHyperLoop>>,
    >,
    sl_source: SelfLoopPortRef,
    sl_target: SelfLoopPortRef,
}

impl SelfLoopEdge {
    pub fn new(
        l_edge: &LEdgeRef,
        sl_source: &SelfLoopPortRef,
        sl_target: &SelfLoopPortRef,
    ) -> SelfLoopEdgeRef {
        let edge_ref = Arc::new(Mutex::new(SelfLoopEdge {
            l_edge: l_edge.clone(),
            sl_hyper_loop: None,
            sl_source: sl_source.clone(),
            sl_target: sl_target.clone(),
        }));

        if let Ok(mut source_guard) = sl_source.lock() {
            source_guard.outgoing_sl_edges_mut().push(edge_ref.clone());
        }
        if let Ok(mut target_guard) = sl_target.lock() {
            target_guard.incoming_sl_edges_mut().push(edge_ref.clone());
        }

        edge_ref
    }

    pub fn l_edge(&self) -> &LEdgeRef {
        &self.l_edge
    }

    pub fn sl_hyper_loop(&self) -> Option<SelfHyperLoopRef> {
        self.sl_hyper_loop.as_ref().and_then(Weak::upgrade)
    }

    pub fn set_sl_hyper_loop(&mut self, sl_loop: &SelfHyperLoopRef) {
        if self.sl_hyper_loop.is_none() {
            self.sl_hyper_loop = Some(Arc::downgrade(sl_loop));
        }
    }

    pub fn sl_source(&self) -> &SelfLoopPortRef {
        &self.sl_source
    }

    pub fn sl_target(&self) -> &SelfLoopPortRef {
        &self.sl_target
    }
}
