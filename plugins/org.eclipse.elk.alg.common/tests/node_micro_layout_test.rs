use std::sync::Once;

use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::force_layout_provider::ForceLayoutProvider;
use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::options::{
    ForceMetaDataProvider, ForceOptions, StressMetaDataProvider, StressOptions,
};
use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::stress::stress_layout_provider::StressLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_alg_mrtree::org::eclipse::elk::alg::mrtree::options::{
    MrTreeMetaDataProvider, MrTreeOptions,
};
use org_eclipse_elk_alg_mrtree::org::eclipse::elk::alg::mrtree::tree_layout_provider::TreeLayoutProvider;
use org_eclipse_elk_alg_radial::org::eclipse::elk::alg::radial::options::{
    RadialMetaDataProvider, RadialOptions,
};
use org_eclipse_elk_alg_radial::org::eclipse::elk::alg::radial::radial_layout_provider::RadialLayoutProvider;
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::options::{
    RectPackingMetaDataProvider, RectPackingOptions,
};
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::rect_packing_layout_provider::RectPackingLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, NodeLabelPlacement, SizeConstraint,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, EnumSet};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkGraphElementRef, ElkLabelRef, ElkNodeRef};

const EPS: f64 = 0.01;

#[derive(Clone, Copy, Debug)]
enum Algorithm {
    Force,
    Layered,
    MrTree,
    Radial,
    RectPacking,
    Stress,
}

impl Algorithm {
    fn all() -> [Algorithm; 6] {
        [
            Algorithm::Force,
            Algorithm::Layered,
            Algorithm::MrTree,
            Algorithm::Radial,
            Algorithm::RectPacking,
            Algorithm::Stress,
        ]
    }

    fn id(self) -> &'static str {
        match self {
            Algorithm::Force => ForceOptions::ALGORITHM_ID,
            Algorithm::Layered => LayeredOptions::ALGORITHM_ID,
            Algorithm::MrTree => MrTreeOptions::ALGORITHM_ID,
            Algorithm::Radial => RadialOptions::ALGORITHM_ID,
            Algorithm::RectPacking => RectPackingOptions::ALGORITHM_ID,
            Algorithm::Stress => StressOptions::ALGORITHM_ID,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Algorithm::Force => "force",
            Algorithm::Layered => "layered",
            Algorithm::MrTree => "mrtree",
            Algorithm::Radial => "radial",
            Algorithm::RectPacking => "rectpacking",
            Algorithm::Stress => "stress",
        }
    }
}

fn init_meta_data() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&ForceMetaDataProvider);
        service.register_layout_meta_data_provider(&StressMetaDataProvider);
        service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
        service.register_layout_meta_data_provider(&MrTreeMetaDataProvider);
        service.register_layout_meta_data_provider(&RadialMetaDataProvider);
        service.register_layout_meta_data_provider(&RectPackingMetaDataProvider);
    });
}

fn set_node_geometry(node: &ElkNodeRef, width: f64, height: f64) {
    let mut node_mut = node.borrow_mut();
    node_mut.connectable().shape().set_dimensions(width, height);
}

fn set_label_geometry(label: &ElkLabelRef, x: f64, y: f64, width: f64, height: f64) {
    let mut label_mut = label.borrow_mut();
    label_mut.shape().set_location(x, y);
    label_mut.shape().set_dimensions(width, height);
}

fn add_label(
    node: &ElkNodeRef,
    text: &str,
    placement: EnumSet<NodeLabelPlacement>,
) -> ElkLabelRef {
    let label = ElkGraphUtil::create_label_with_text(text, Some(ElkGraphElementRef::Node(node.clone())));
    set_label_geometry(&label, 0.0, 0.0, 10.0, 10.0);
    label
        .borrow_mut()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::NODE_LABELS_PLACEMENT, Some(placement));
    label
}

fn build_graph() -> (ElkNodeRef, ElkNodeRef) {
    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    set_node_geometry(&node, 100.0, 100.0);

    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(
            CoreOptions::NODE_SIZE_CONSTRAINTS,
            Some(EnumSet::of(&[SizeConstraint::MinimumSize])),
        );
    node.borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(
            CoreOptions::NODE_SIZE_MINIMUM,
            Some(KVector::with_values(101.0, 102.0)),
        );

    add_label(&node, "A", NodeLabelPlacement::inside_top_center());
    add_label(&node, "B", NodeLabelPlacement::inside_bottom_right());
    add_label(&node, "C", NodeLabelPlacement::outside_bottom_center());
    add_label(
        &node,
        "D",
        EnumSet::of(&[
            NodeLabelPlacement::Outside,
            NodeLabelPlacement::VCenter,
            NodeLabelPlacement::HLeft,
        ]),
    );

    (graph, node)
}

fn configure_omit_micro_layout(
    algorithm: Algorithm,
    graph: &ElkNodeRef,
    omit_requested: bool,
) -> bool {
    let omit_effective = omit_requested && !matches!(algorithm, Algorithm::Layered);
    let mut graph_mut = graph.borrow_mut();
    let props = graph_mut.connectable().shape().graph_element().properties_mut();
    props.set_property(CoreOptions::ALGORITHM, Some(algorithm.id().to_string()));
    if omit_effective {
        props.set_property(CoreOptions::OMIT_NODE_MICRO_LAYOUT, Some(true));
    } else {
        props.set_property(CoreOptions::OMIT_NODE_MICRO_LAYOUT, None::<bool>);
    }
    omit_effective
}

fn run_layout(algorithm: Algorithm, graph: &ElkNodeRef) {
    let mut monitor = BasicProgressMonitor::new();
    match algorithm {
        Algorithm::Force => {
            let mut provider = ForceLayoutProvider::new();
            provider.layout(graph, &mut monitor);
        }
        Algorithm::Layered => {
            let mut provider = LayeredLayoutProvider::new();
            provider.layout(graph, &mut monitor);
        }
        Algorithm::MrTree => {
            let mut provider = TreeLayoutProvider::new();
            provider.layout(graph, &mut monitor);
        }
        Algorithm::Radial => {
            let mut provider = RadialLayoutProvider::new();
            provider.layout(graph, &mut monitor);
        }
        Algorithm::RectPacking => {
            let mut provider = RectPackingLayoutProvider::new();
            provider.layout(graph, &mut monitor);
        }
        Algorithm::Stress => {
            let mut provider = StressLayoutProvider::new();
            provider.layout(graph, &mut monitor);
        }
    }
}

fn node_dimensions(node: &ElkNodeRef) -> (f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.width(), shape.height())
}

fn label_by_text(node: &ElkNodeRef, text: &str) -> ElkLabelRef {
    let labels: Vec<ElkLabelRef> = {
        let mut node_mut = node.borrow_mut();
        node_mut
            .connectable()
            .shape()
            .graph_element()
            .labels()
            .iter()
            .cloned()
            .collect()
    };
    labels
        .into_iter()
        .find(|label| label.borrow().text() == text)
        .unwrap_or_else(|| panic!("label {text} not found"))
}

fn label_xy(label: &ElkLabelRef) -> (f64, f64) {
    let mut label_mut = label.borrow_mut();
    let shape = label_mut.shape();
    (shape.x(), shape.y())
}

fn assert_close(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= EPS,
        "{context}: actual={actual}, expected={expected}, eps={EPS}"
    );
}

fn verify_minimum_size(node: &ElkNodeRef, omit_micro_layout: bool, context: &str) {
    let (width, height) = node_dimensions(node);
    if omit_micro_layout {
        assert_close(width, 100.0, &format!("{context} width"));
        assert_close(height, 100.0, &format!("{context} height"));
    } else {
        assert_close(width, 101.0, &format!("{context} width"));
        assert_close(height, 102.0, &format!("{context} height"));
    }
}

fn verify_label_positions(node: &ElkNodeRef, omit_micro_layout: bool, context: &str) {
    if omit_micro_layout {
        for label in [
            label_by_text(node, "A"),
            label_by_text(node, "B"),
            label_by_text(node, "C"),
            label_by_text(node, "D"),
        ] {
            let (x, _) = label_xy(&label);
            assert_close(x, 0.0, &format!("{context} label x"));
        }
        return;
    }

    let before_center_threshold = 40.0;
    let after_center_threshold = 60.0;

    let (node_width, node_height) = node_dimensions(node);

    let (a_x, a_y) = label_xy(&label_by_text(node, "A"));
    assert!(
        a_y > 0.0,
        "{context} label A y expected > 0, got {a_y}"
    );
    let _ = a_x;

    let (b_x, b_y) = label_xy(&label_by_text(node, "B"));
    assert!(
        b_x > after_center_threshold && b_x < node_width,
        "{context} label B x expected > {after_center_threshold} and < {node_width}, got {b_x}"
    );
    assert!(
        b_y > after_center_threshold && b_y < node_height,
        "{context} label B y expected > {after_center_threshold} and < {node_height}, got {b_y}"
    );

    let (c_x, c_y) = label_xy(&label_by_text(node, "C"));
    assert!(
        c_x > before_center_threshold && c_x < after_center_threshold,
        "{context} label C x expected between {before_center_threshold} and {after_center_threshold}, got {c_x}"
    );
    assert!(
        c_y > node_height,
        "{context} label C y expected > {node_height}, got {c_y}"
    );

    let (d_x, d_y) = label_xy(&label_by_text(node, "D"));
    assert!(
        d_x < 0.0,
        "{context} label D x expected < 0, got {d_x}"
    );
    assert!(
        d_y > before_center_threshold && d_y < after_center_threshold,
        "{context} label D y expected between {before_center_threshold} and {after_center_threshold}, got {d_y}"
    );
}

#[test]
fn test_minimum_size() {
    init_meta_data();
    for algorithm in Algorithm::all() {
        for omit_requested in [false, true] {
            let (graph, node) = build_graph();
            let omit_effective = configure_omit_micro_layout(algorithm, &graph, omit_requested);
            run_layout(algorithm, &graph);
            let context = format!(
                "algorithm={} omit_requested={omit_requested}",
                algorithm.name()
            );
            verify_minimum_size(&node, omit_effective, &context);
        }
    }
}

#[test]
fn test_node_label_positions() {
    init_meta_data();
    for algorithm in Algorithm::all() {
        for omit_requested in [false, true] {
            let (graph, node) = build_graph();
            let omit_effective = configure_omit_micro_layout(algorithm, &graph, omit_requested);
            run_layout(algorithm, &graph);
            let context = format!(
                "algorithm={} omit_requested={omit_requested}",
                algorithm.name()
            );
            verify_label_positions(&node, omit_effective, &context);
        }
    }
}
