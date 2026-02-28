use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMargin, ElkPadding};
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IndividualSpacings;

#[test]
fn individual_spacings_serializes_object_values() {
    LayoutMetaDataService::get_instance();

    let mut spacings = IndividualSpacings::new();
    let padding = ElkPadding::with_values(1.0, 2.0, 3.0, 4.0);
    let margin = ElkMargin::with_values(5.0, 6.0, 7.0, 8.0);

    spacings
        .properties_mut()
        .set_property(CoreOptions::NODE_LABELS_PADDING, Some(padding.clone()));
    spacings
        .properties_mut()
        .set_property(CoreOptions::SPACING_PORTS_SURROUNDING, Some(margin.clone()));

    let serialized = spacings.to_string();
    assert!(serialized.contains("org.eclipse.elk.nodeLabels.padding"));
    assert!(serialized.contains("[top=1,left=4,bottom=3,right=2]"));
    assert!(serialized.contains("org.eclipse.elk.spacing.portsSurrounding"));
    assert!(serialized.contains("[top=5,left=8,bottom=7,right=6]"));

    let mut parsed = IndividualSpacings::new();
    parsed
        .parse(&serialized)
        .expect("parse individual spacings");

    let parsed_padding = parsed
        .properties_mut()
        .get_property(CoreOptions::NODE_LABELS_PADDING)
        .expect("padding parsed");
    let parsed_margin = parsed
        .properties_mut()
        .get_property(CoreOptions::SPACING_PORTS_SURROUNDING)
        .expect("margin parsed");

    assert_eq!(padding, parsed_padding);
    assert_eq!(margin, parsed_margin);
}
