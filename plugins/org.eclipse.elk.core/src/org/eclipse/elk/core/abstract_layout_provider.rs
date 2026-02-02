use crate::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use crate::org::eclipse::elk::core::topdown_layout_provider::ITopdownLayoutProvider;
use crate::org::eclipse::elk::core::testing::IWhiteBoxTestable;

pub trait AbstractLayoutProvider: IGraphLayoutEngine + Send {
    fn initialize(&mut self, _parameter: &str) {}

    fn dispose(&mut self) {}

    fn as_topdown_layout_provider(&self) -> Option<&dyn ITopdownLayoutProvider> {
        None
    }

    fn as_white_box_testable(&mut self) -> Option<&mut dyn IWhiteBoxTestable> {
        None
    }
}
