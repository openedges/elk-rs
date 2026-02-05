use org_eclipse_elk_alg_graphviz_dot::org::eclipse::elk::alg::graphviz::dot::transform::Command;
use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

pub struct GraphvizLayoutProvider {
    command: Command,
}

impl GraphvizLayoutProvider {
    pub const DOT: &'static str = "DOT";
    pub const NEATO: &'static str = "NEATO";
    pub const FDP: &'static str = "FDP";
    pub const TWOPI: &'static str = "TWOPI";
    pub const CIRCO: &'static str = "CIRCO";

    pub fn new() -> Self {
        GraphvizLayoutProvider {
            command: Command::Invalid,
        }
    }

    pub fn initialize(&mut self, parameter: &str) {
        self.command = Command::parse(parameter);
    }

    pub fn dispose(&mut self) {}
}

impl Default for GraphvizLayoutProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IGraphLayoutEngine for GraphvizLayoutProvider {
    fn layout(&mut self, parent_node: &ElkNodeRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        if self.command == Command::Invalid {
            panic!("The Graphviz layout provider is not initialized.");
        }

        let label = self.command.literal();
        let task = format!("Graphviz layout ({})", label);
        progress_monitor.begin(&task, 1.0);

        let children_empty = {
            let mut parent_mut = parent_node.borrow_mut();
            parent_mut.children().is_empty()
        };
        if children_empty {
            progress_monitor.done();
            return;
        }

        // TODO: port Graphviz dot transformation and external Graphviz invocation.

        progress_monitor.done();
    }
}

impl AbstractLayoutProvider for GraphvizLayoutProvider {}
