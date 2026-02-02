use std::any::Any;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;

use crate::org::eclipse::elk::core::service::LayoutMapping;

pub trait IDiagramLayoutConnector {
    fn build_layout_graph(
        &self,
        workbench_part: Option<&dyn Any>,
        diagram_part: Option<&dyn Any>,
    ) -> Option<LayoutMapping>;

    fn apply_layout(&self, mapping: &mut LayoutMapping, settings: &MapPropertyHolder);
}
