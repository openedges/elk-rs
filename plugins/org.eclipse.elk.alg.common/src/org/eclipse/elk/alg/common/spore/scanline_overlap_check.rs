use std::cmp::Ordering;

use crate::org::eclipse::elk::alg::common::compaction::{Scanline, ScanlineEventHandler};
use crate::org::eclipse::elk::alg::common::spore::internal_properties::InternalProperties;
use crate::org::eclipse::elk::alg::common::spore::i_overlap_handler::IOverlapHandler;
use crate::org::eclipse::elk::alg::common::spore::node::Node;
use crate::org::eclipse::elk::alg::common::utils::SVGImage;

pub struct ScanlineOverlapCheck<H: IOverlapHandler> {
    overlap_handler: H,
    svg: SVGImage,
}

impl<H: IOverlapHandler> ScanlineOverlapCheck<H> {
    pub fn new(overlap_handler: H, mut svg: SVGImage) -> Self {
        svg.add_groups(&["n", "o", "e"]);
        ScanlineOverlapCheck { overlap_handler, svg }
    }

    pub fn sweep(&mut self, nodes: &[Node]) {
        self.svg.clear_group("n");
        self.svg.clear_group("o");
        self.svg.clear_group("e");

        let mut points: Vec<Timestamp> = Vec::new();
        for (idx, node) in nodes.iter().enumerate() {
            points.push(Timestamp { index: idx, low: true });
            points.push(Timestamp { index: idx, low: false });
            self.svg
                .g("n")
                .add_rect(&node.rect, "stroke=\"black\" stroke-width=\"1\" fill=\"none\"");
        }

        self.svg.isave();

        let mut handler = OverlapsScanlineHandler::new(nodes, &self.overlap_handler, &mut self.svg);
        handler.init();
        Scanline::execute(points, overlaps_scanline_comparator(nodes), &mut handler);

        self.svg.isave();
    }
}

#[derive(Clone, Copy)]
struct Timestamp {
    index: usize,
    low: bool,
}

fn overlaps_scanline_comparator(nodes: &[Node]) -> impl Fn(&Timestamp, &Timestamp) -> Ordering + '_ {
    move |p1: &Timestamp, p2: &Timestamp| {
        let mut y1 = nodes[p1.index].rect.y;
        if !p1.low {
            y1 += nodes[p1.index].rect.height;
        }
        let mut y2 = nodes[p2.index].rect.y;
        if !p2.low {
            y2 += nodes[p2.index].rect.height;
        }
        let cmp = y1.partial_cmp(&y2).unwrap_or(Ordering::Equal);
        if cmp == Ordering::Equal {
            if !p1.low && p2.low {
                return Ordering::Less;
            }
            if !p2.low && p1.low {
                return Ordering::Greater;
            }
        }
        cmp
    }
}

struct OverlapsScanlineHandler<'a, H: IOverlapHandler> {
    nodes: &'a [Node],
    overlap_handler: &'a H,
    svg: &'a mut SVGImage,
    intervals: Vec<usize>,
}

impl<'a, H: IOverlapHandler> OverlapsScanlineHandler<'a, H> {
    fn new(nodes: &'a [Node], overlap_handler: &'a H, svg: &'a mut SVGImage) -> Self {
        OverlapsScanlineHandler {
            nodes,
            overlap_handler,
            svg,
            intervals: Vec::new(),
        }
    }

    fn init(&mut self) {
        self.intervals.clear();
    }

    fn insert(&mut self, index: usize) {
        let node = &self.nodes[index];
        let insert_pos = self
            .intervals
            .iter()
            .position(|other| compare_nodes(node, &self.nodes[*other]) == Ordering::Less)
            .unwrap_or(self.intervals.len());
        self.intervals.insert(insert_pos, index);

        let mut overlaps_found = false;
        for &other_idx in &self.intervals {
            let other = &self.nodes[other_idx];
            if overlap(node, other) {
                self.overlap_handler.handle(node, other);
                self.svg
                    .g("o")
                    .add_rect(&node.rect, "stroke=\"none\" fill=\"red\" opacity=\"0.18\"");
                self.svg
                    .g("o")
                    .add_rect(&other.rect, "stroke=\"none\" fill=\"red\" opacity=\"0.18\"");
                self.svg.g("e").add_line_with_attrs(
                    node.vertex.x,
                    node.vertex.y,
                    other.vertex.x,
                    other.vertex.y,
                    "stroke=\"blue\"",
                );
                overlaps_found = true;
            } else if overlaps_found {
                break;
            }
        }
    }

    fn delete(&mut self, index: usize) {
        if let Some(pos) = self.intervals.iter().position(|val| *val == index) {
            self.intervals.remove(pos);
        }
    }
}

impl<'a, H: IOverlapHandler> ScanlineEventHandler<Timestamp> for OverlapsScanlineHandler<'a, H> {
    fn handle(&mut self, point: &Timestamp) {
        if point.low {
            self.insert(point.index);
        } else {
            self.delete(point.index);
        }
    }
}

fn compare_nodes(n1: &Node, n2: &Node) -> Ordering {
    let cmp = n1.rect.x.partial_cmp(&n2.rect.x).unwrap_or(Ordering::Equal);
    if cmp != Ordering::Equal {
        return cmp;
    }
    let cmp = n1
        .original_vertex
        .x
        .partial_cmp(&n2.original_vertex.x)
        .unwrap_or(Ordering::Equal);
    if cmp != Ordering::Equal {
        return cmp;
    }
    n1.original_vertex
        .y
        .partial_cmp(&n2.original_vertex.y)
        .unwrap_or(Ordering::Equal)
}

fn overlap(n1: &Node, n2: &Node) -> bool {
    if std::ptr::eq(n1, n2) {
        return false;
    }
    fuzzy_compare(
        n1.rect.x,
        n2.rect.x + n2.rect.width,
        InternalProperties::FUZZINESS,
    ) < 0
        && fuzzy_compare(
            n2.rect.x,
            n1.rect.x + n1.rect.width,
            InternalProperties::FUZZINESS,
        ) < 0
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
