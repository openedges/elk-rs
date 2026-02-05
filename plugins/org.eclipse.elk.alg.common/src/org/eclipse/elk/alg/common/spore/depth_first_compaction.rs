use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;

use crate::org::eclipse::elk::alg::common::spore::internal_properties::InternalProperties;
use crate::org::eclipse::elk::alg::common::spore::node::Node;
use crate::org::eclipse::elk::alg::common::tree::Tree;
use crate::org::eclipse::elk::alg::common::utils::SVGImage;

pub struct DepthFirstCompaction;

impl DepthFirstCompaction {
    pub fn compact(tree: &mut Tree<Node>, orthogonal: bool, debug_output_file: Option<&str>) {
        let mut svg = SVGImage::new(debug_output_file);
        debug_out(&mut svg, tree, None);
        let root_ptr: *const Tree<Node> = tree;
        compact_tree(tree, root_ptr, orthogonal, &mut svg);
    }

    pub fn compact_without_debug(tree: &mut Tree<Node>, orthogonal: bool) {
        Self::compact(tree, orthogonal, None);
    }
}

fn compact_tree(
    tree: &mut Tree<Node>,
    root_ptr: *const Tree<Node>,
    orthogonal: bool,
    svg: &mut SVGImage,
) {
    let root = unsafe { &*root_ptr };
    for child in &mut tree.children {
        compact_tree(child, root_ptr, orthogonal, svg);
    }

    for idx in 0..tree.children.len() {
        let (mut compaction_vector, min_underlap) = {
            let child_ref = &tree.children[idx];
            let mut compaction_vector = tree.node.vertex;
            compaction_vector.sub(&child_ref.node.vertex);

            if orthogonal {
                let rt = tree.node.rect;
                let rc = child_ref.node.rect;
                if compaction_vector.x.abs() >= compaction_vector.y.abs() {
                    compaction_vector.y = 0.0;
                    if rc.y + rc.height > rt.y && rc.y < rt.y + rt.height {
                        let dist = (rt.x - (rc.x + rc.width)).max(rc.x - (rt.x + rt.width));
                        compaction_vector.scale_to_length(dist);
                    }
                } else {
                    compaction_vector.x = 0.0;
                    if rc.x + rc.width > rt.x && rc.x < rt.x + rt.width {
                        let dist = (rt.y - (rc.y + rc.height)).max(rc.y - (rt.y + rt.height));
                        compaction_vector.scale_to_length(dist);
                    }
                }
            } else {
                let underlap = tree.node.underlap(&child_ref.node);
                compaction_vector.scale_to_length(underlap);
            }

            let min_underlap = compaction_vector.length();
            let min_underlap = get_min_underlap(root, child_ref, min_underlap, &compaction_vector);
            (compaction_vector, min_underlap)
        };

        compaction_vector.scale_to_length(min_underlap);
        translate_subtree(&mut tree.children[idx], &compaction_vector);

        debug_out(svg, root, Some(&tree.children[idx]));
    }
}

fn get_min_underlap(
    tree: &Tree<Node>,
    child: &Tree<Node>,
    current_min_underlap: f64,
    compaction_vector: &KVector,
) -> f64 {
    let mut min_underlap = current_min_underlap.min(min_underlap_with_subtree(
        &tree.node,
        child,
        current_min_underlap,
        compaction_vector,
    ));

    for c in &tree.children {
        if !std::ptr::eq(c, child) {
            min_underlap = min_underlap.min(get_min_underlap(c, child, min_underlap, compaction_vector));
        }
    }

    min_underlap
}

fn min_underlap_with_subtree(
    root: &Node,
    tree: &Tree<Node>,
    current_min_underlap: f64,
    compaction_vector: &KVector,
) -> f64 {
    let mut min_underlap = current_min_underlap;
    for child in &tree.children {
        let c = &child.node;
        if root.touches(c) {
            if (fuzzy_compare(c.rect.x, root.rect.x + root.rect.width, InternalProperties::FUZZINESS)
                == 0
                && compaction_vector.x < 0.0)
                || (fuzzy_compare(
                    c.rect.x + c.rect.width,
                    root.rect.x,
                    InternalProperties::FUZZINESS,
                ) == 0
                    && compaction_vector.x > 0.0)
                || (fuzzy_compare(
                    c.rect.y,
                    root.rect.y + root.rect.height,
                    InternalProperties::FUZZINESS,
                ) == 0
                    && compaction_vector.y < 0.0)
                || (fuzzy_compare(
                    c.rect.y + c.rect.height,
                    root.rect.y,
                    InternalProperties::FUZZINESS,
                ) == 0
                    && compaction_vector.y > 0.0)
            {
                min_underlap = 0.0;
                break;
            }
        } else {
            min_underlap = min_underlap.min(root.distance(c, compaction_vector));
        }

        min_underlap = min_underlap.min(min_underlap_with_subtree(
            root,
            child,
            min_underlap,
            compaction_vector,
        ));
    }

    min_underlap
}

fn translate_subtree(tree: &mut Tree<Node>, compaction_vector: &KVector) {
    tree.node.translate(compaction_vector);
    for child in &mut tree.children {
        translate_subtree(child, compaction_vector);
    }
}

fn draw_tree(tree: &Tree<Node>, svg: &mut SVGImage, mark: Option<&Tree<Node>>) {
    svg.g("rects")
        .add_rect(&tree.node.rect, "fill=\"none\" stroke=\"black\"");
    svg.g("centers")
        .add_circle_with_attrs(tree.node.vertex.x, tree.node.vertex.y, 6.0, "fill=\"black\"");
    for child in &tree.children {
        if mark.map_or(false, |m| std::ptr::eq(m, child)) {
            svg.g("edges").add_line_with_attrs(
                tree.node.vertex.x,
                tree.node.vertex.y,
                child.node.vertex.x,
                child.node.vertex.y,
                "stroke=\"red\" stroke-width=\"5\" stroke-dasharray=\"8,8\"",
            );
        } else {
            svg.g("edges").add_line_with_attrs(
                tree.node.vertex.x,
                tree.node.vertex.y,
                child.node.vertex.x,
                child.node.vertex.y,
                "stroke=\"blue\" stroke-width=\"2\"",
            );
        }
        draw_tree(child, svg, mark);
    }
}

fn debug_out(svg: &mut SVGImage, root: &Tree<Node>, mark: Option<&Tree<Node>>) {
    svg.clear();
    svg.add_groups(&["rects", "root", "edges", "centers", "marks"]);
    svg.g("root")
        .add_rect(&root.node.rect, "fill=\"#C4E3F3\" stroke=\"black\"");
    draw_tree(root, svg, mark);
    svg.isave();
}

fn fuzzy_compare(a: f64, b: f64, eps: f64) -> i32 {
    if (a - b).abs() <= eps {
        0
    } else if a < b {
        -1
    } else {
        1
    }
}
