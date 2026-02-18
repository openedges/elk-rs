use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::depth_first_compaction::DepthFirstCompaction;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::internal_properties::InternalProperties;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::node::Node;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::tree::Tree;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::utils::SVGImage;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, IElkProgressMonitor};

use crate::org::eclipse::elk::alg::spore::graph::Graph;
use crate::org::eclipse::elk::alg::spore::spore_phases::SPOrEPhases;

pub struct ShrinkTreeCompactionPhase;

impl ShrinkTreeCompactionPhase {
    pub fn new() -> Self {
        ShrinkTreeCompactionPhase
    }
}

impl Default for ShrinkTreeCompactionPhase {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<SPOrEPhases, Graph> for ShrinkTreeCompactionPhase {
    fn process(&mut self, graph: &mut Graph, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Shrinking tree compaction", 1.0);

        let debug = graph
            .get_property(InternalProperties::DEBUG_SVG)
            .unwrap_or(false);

        let Some(tree) = graph.tree.as_mut() else {
            progress_monitor.done();
            return;
        };
        if debug {
            if let Some(path) = ElkUtil::debug_folder_path(&["spore"]) {
                debug_out(tree, &format!("{}30Tree", path));
                DepthFirstCompaction::compact(
                    tree,
                    graph.orthogonal_compaction,
                    Some(&format!("{}60compaction", path)),
                );
            } else {
                DepthFirstCompaction::compact_without_debug(tree, graph.orthogonal_compaction);
            }
        } else {
            DepthFirstCompaction::compact_without_debug(tree, graph.orthogonal_compaction);
        }

        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &Graph,
    ) -> Option<LayoutProcessorConfiguration<SPOrEPhases, Graph>> {
        Some(LayoutProcessorConfiguration::create())
    }
}

fn debug_out(tree: &Tree<Node>, file_name: &str) {
    let mut svg = SVGImage::new(Some(file_name));
    svg.clear();
    svg.add_circle_with_attrs(
        tree.node.vertex.x,
        tree.node.vertex.y,
        10.0,
        "fill=\"lime\"",
    );
    draw(tree, &mut svg);
    svg.save();
    svg.debug = false;
}

fn draw(tree: &Tree<Node>, svg: &mut SVGImage) {
    svg.add_rect(&tree.node.rect, "fill=\"none\" stroke=\"black\"");
    for child in &tree.children {
        svg.add_line_with_attrs(
            tree.node.vertex.x,
            tree.node.vertex.y,
            child.node.vertex.x,
            child.node.vertex.y,
            "stroke=\"blue\"",
        );
        let mut cv = tree.node.vertex;
        cv.sub(&child.node.vertex);
        cv.scale_to_length(tree.node.distance(&child.node, &cv));
        svg.add_line_with_attrs(
            child.node.vertex.x,
            child.node.vertex.y,
            child.node.vertex.x + cv.x,
            child.node.vertex.y + cv.y,
            "stroke=\"orange\"",
        );
        draw(child, svg);
    }
}
