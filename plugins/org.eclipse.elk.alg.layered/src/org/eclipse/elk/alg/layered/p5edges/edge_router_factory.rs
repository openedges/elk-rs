use rustc_hash::FxHashMap;
use std::sync::{Arc, LazyLock};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;

use crate::org::eclipse::elk::alg::layered::graph::LGraph;
use crate::org::eclipse::elk::alg::layered::p5edges::orthogonal_edge_router::OrthogonalEdgeRouter;
use crate::org::eclipse::elk::alg::layered::p5edges::polyline_edge_router::PolylineEdgeRouter;
use crate::org::eclipse::elk::alg::layered::p5edges::splines::spline_edge_router::SplineEdgeRouter;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

static FACTORY_CACHE: LazyLock<Mutex<FxHashMap<EdgeRouting, Arc<EdgeRouterFactory>>>> =
    LazyLock::new(|| Mutex::new(FxHashMap::default()));

pub struct EdgeRouterFactory {
    edge_routing: EdgeRouting,
}

impl EdgeRouterFactory {
    pub fn factory_for(edge_routing: EdgeRouting) -> Arc<EdgeRouterFactory> {
        let mut cache = FACTORY_CACHE.lock();        if let Some(factory) = cache.get(&edge_routing) {
            return factory.clone();
        }
        let factory = Arc::new(EdgeRouterFactory { edge_routing });
        cache.insert(edge_routing, factory.clone());
        factory
    }
}

impl ILayoutPhaseFactory<LayeredPhases, LGraph> for EdgeRouterFactory {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<LayeredPhases, LGraph>> {
        match self.edge_routing {
            EdgeRouting::Polyline => Box::new(PolylineEdgeRouter::new()),
            EdgeRouting::Splines => Box::new(SplineEdgeRouter::new()),
            _ => Box::new(OrthogonalEdgeRouter::new()),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
