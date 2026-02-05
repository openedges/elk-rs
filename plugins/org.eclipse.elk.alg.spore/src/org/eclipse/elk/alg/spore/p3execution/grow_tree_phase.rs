use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::internal_properties::InternalProperties;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::spore::node::Node;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::tree::Tree;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::utils::{SVGImage, Utils};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, IElkProgressMonitor};

use crate::org::eclipse::elk::alg::spore::graph::Graph;
use crate::org::eclipse::elk::alg::spore::spore_phases::SPOrEPhases;

pub struct GrowTreePhase;

impl GrowTreePhase {
    pub fn new() -> Self {
        GrowTreePhase
    }
}

impl Default for GrowTreePhase {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<SPOrEPhases, Graph> for GrowTreePhase {
    fn process(&mut self, graph: &mut Graph, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Grow Tree", 1.0);

        let debug = graph
            .get_property(InternalProperties::DEBUG_SVG)
            .unwrap_or(false);

        let Some(root) = graph.tree.as_mut() else {
            progress_monitor.done();
            return;
        };
        let mut svg = if debug {
            let file = ElkUtil::debug_folder_path(&["spore"]).map(|path| format!("{}40or", path));
            let mut image = SVGImage::new(file.as_deref());
            image.add_groups(&["n", "e", "o"]);
            image
        } else {
            SVGImage::new(None)
        };

        if debug {
            debug_out(&mut svg, root, None);
        }

        let mut overlaps_existed = false;
        let root_ptr: *const Tree<Node> = root;
        grow_at(root, root_ptr, &mut svg, debug, &mut overlaps_existed);
        graph.set_property(InternalProperties::OVERLAPS_EXISTED, Some(overlaps_existed));

        progress_monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &Graph,
    ) -> Option<LayoutProcessorConfiguration<SPOrEPhases, Graph>> {
        Some(LayoutProcessorConfiguration::create())
    }
}

fn grow_at(
    tree: &mut Tree<Node>,
    root_ptr: *const Tree<Node>,
    svg: &mut SVGImage,
    debug: bool,
    overlaps_existed: &mut bool,
) {
    for child in &mut tree.children {
        let mut delta = tree.node.vertex;
        delta.sub(&tree.node.original_vertex);
        child.node.translate(&delta);

        let t = Utils::overlap(&tree.node.rect, &child.node.rect);
        if t > 1.0 {
            *overlaps_existed = true;
        }

        let mut direction = child.node.original_vertex;
        direction.sub(&tree.node.original_vertex);
        direction.scale(t);
        let mut new_center = tree.node.vertex;
        new_center.add(&direction);
        child.node.set_center_position(&new_center);

        if debug {
            let root = unsafe { &*root_ptr };
            let child_ptr: *const Tree<Node> = child;
            debug_out(svg, root, Some(unsafe { &*child_ptr }));
        }

        grow_at(child, root_ptr, svg, debug, overlaps_existed);
    }
}

fn debug_out(svg: &mut SVGImage, root: &Tree<Node>, current: Option<&Tree<Node>>) {
    svg.clear_group("n");
    svg.clear_group("e");
    svg.clear_group("o");
    draw_tree(root, svg);
    svg.g("n")
        .add_rect(&root.node.rect, "fill=\"blue\" stroke=\"none\" opacity=\"0.2\"");
    if let Some(current) = current {
        svg.g("o")
            .add_rect(&current.node.rect, "fill=\"red\" stroke=\"none\" opacity=\"0.2\"");
    }
    svg.isave();
}

fn draw_tree(tree: &Tree<Node>, svg: &mut SVGImage) {
    svg.g("n")
        .add_rect(&tree.node.rect, "fill=\"none\" stroke=\"black\"");
    for child in &tree.children {
        svg.g("e").add_line_with_attrs(
            tree.node.vertex.x,
            tree.node.vertex.y,
            child.node.vertex.x,
            child.node.vertex.y,
            "stroke=\"blue\"",
        );
        draw_tree(child, svg);
    }
}
