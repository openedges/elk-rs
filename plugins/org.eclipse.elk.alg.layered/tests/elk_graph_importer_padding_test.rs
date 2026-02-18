use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_importer::ElkGraphImporter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::transform::elk_graph_transformer::OriginStore;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{CoreOptions, NodeLabelPlacement};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IndividualSpacings};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef;

fn approx_eq(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn importer_adds_inside_node_label_padding_to_graph_padding() {
    LayoutMetaDataService::get_instance();
    initialize_plain_java_layout();

    let graph = ElkGraphUtil::create_graph();
    graph
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(
            CoreOptions::PADDING,
            Some(ElkPadding::with_values(1.0, 2.0, 3.0, 4.0)),
        );
    let mut spacing_overrides = IndividualSpacings::new();
    spacing_overrides.properties_mut().set_property(
        CoreOptions::NODE_LABELS_PADDING,
        Some(ElkPadding::with_values(2.0, 3.0, 4.0, 5.0)),
    );
    spacing_overrides
        .properties_mut()
        .set_property(CoreOptions::SPACING_LABEL_LABEL, Some(1.0));
    graph
        .borrow_mut()
        .connectable()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(CoreOptions::SPACING_INDIVIDUAL, Some(spacing_overrides));

    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Node(graph.clone())));
    label.borrow_mut().shape().set_dimensions(10.0, 5.0);
    label
        .borrow_mut()
        .shape()
        .graph_element()
        .properties_mut()
        .set_property(
            CoreOptions::NODE_LABELS_PLACEMENT,
            Some(EnumSet::of(&[
                NodeLabelPlacement::Inside,
                NodeLabelPlacement::VTop,
                NodeLabelPlacement::HCenter,
            ])),
        );

    let mut origin_store = OriginStore::new();
    let mut importer = ElkGraphImporter::new(&mut origin_store);
    let lgraph = importer.import_graph(&graph);

    let mut graph_guard = lgraph.lock().expect("lgraph lock");
    let padding = graph_guard.padding().clone();
    drop(graph_guard);

    // Top: base(1) + labelHeight(5) + nodeLabelsPadding.top(2) + cellGap(2)
    approx_eq(padding.top, 10.0);
    approx_eq(padding.right, 2.0);
    approx_eq(padding.bottom, 3.0);
    approx_eq(padding.left, 4.0);
}
