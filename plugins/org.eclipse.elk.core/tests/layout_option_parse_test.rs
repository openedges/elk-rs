use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector, KVectorChain};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    ContentAlignment, CoreOptions, Direction, EdgeCoords, EdgeRouting, HierarchyHandling,
    NodeLabelPlacement, PortLabelPlacement, ShapeCoords,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, IndividualSpacings};

#[test]
fn parse_object_options_kvector_and_chain() {
    let service = LayoutMetaDataService::get_instance();

    let position = service
        .get_option_data_by_suffix("position")
        .expect("position option");
    let parsed = position
        .parse_value("(1.5, 2.5)")
        .expect("position parsed");
    let vector = parsed.downcast_ref::<KVector>().expect("KVector");
    assert_eq!(vector.x, 1.5);
    assert_eq!(vector.y, 2.5);

    let bend_points = service
        .get_option_data_by_suffix("bendPoints")
        .expect("bendPoints option");
    let parsed = bend_points
        .parse_value("(1,2; 3,4)")
        .expect("bend points parsed");
    let chain = parsed
        .downcast_ref::<KVectorChain>()
        .expect("KVectorChain");
    assert_eq!(chain.len(), 2);
    assert_eq!(chain.get(0).x, 1.0);
    assert_eq!(chain.get(0).y, 2.0);
    assert_eq!(chain.get(1).x, 3.0);
    assert_eq!(chain.get(1).y, 4.0);
}

#[test]
fn parse_object_options_padding_and_margin() {
    let service = LayoutMetaDataService::get_instance();

    let padding_option = service
        .get_option_data_by_suffix("nodeLabels.padding")
        .expect("nodeLabels.padding option");
    let parsed = padding_option
        .parse_value("[top=1,left=2,bottom=3,right=4]")
        .expect("padding parsed");
    let padding = parsed
        .downcast_ref::<ElkPadding>()
        .expect("ElkPadding");
    assert_eq!(padding.top, 1.0);
    assert_eq!(padding.right, 4.0);
    assert_eq!(padding.bottom, 3.0);
    assert_eq!(padding.left, 2.0);

    let margin_option = service
        .get_option_data_by_suffix("spacing.portsSurrounding")
        .expect("portsSurrounding option");
    let parsed = margin_option
        .parse_value("[top=5,left=6,bottom=7,right=8]")
        .expect("margin parsed");
    let margin = parsed.downcast_ref::<ElkMargin>().expect("ElkMargin");
    assert_eq!(margin.top, 5.0);
    assert_eq!(margin.right, 8.0);
    assert_eq!(margin.bottom, 7.0);
    assert_eq!(margin.left, 6.0);
}

#[test]
fn parse_object_option_individual_spacings() {
    let service = LayoutMetaDataService::get_instance();
    let option = service
        .get_option_data_by_suffix("spacing.individual")
        .expect("individual spacings option");

    let serialized = "nodeNode:10;,;nodeLabels.padding:[top=1,left=2,bottom=3,right=4]";
    let parsed = option
        .parse_value(serialized)
        .expect("individual spacings parsed");
    let spacings = parsed
        .downcast_ref::<IndividualSpacings>()
        .expect("IndividualSpacings");

    let mut spacings = spacings.clone();
    let node_node = spacings
        .properties_mut()
        .get_property(CoreOptions::SPACING_NODE_NODE)
        .expect("nodeNode spacing");
    assert_eq!(node_node, 10.0);

    let padding = spacings
        .properties_mut()
        .get_property(CoreOptions::NODE_LABELS_PADDING)
        .expect("nodeLabels.padding");
    assert_eq!(padding, ElkPadding::with_values(1.0, 4.0, 3.0, 2.0));
}

#[test]
fn parse_enum_and_enumset_options() {
    let service = LayoutMetaDataService::get_instance();

    let direction_option = service
        .get_option_data_by_suffix("direction")
        .expect("direction option");
    let parsed = direction_option
        .parse_value("DOWN")
        .expect("direction parsed");
    let direction = parsed
        .downcast_ref::<Direction>()
        .expect("Direction");
    assert_eq!(*direction, Direction::Down);

    let parsed = direction_option
        .parse_value("3")
        .expect("direction ordinal parsed");
    let direction = parsed
        .downcast_ref::<Direction>()
        .expect("Direction");
    assert_eq!(*direction, Direction::Down);

    let placement_option = service
        .get_option_data_by_suffix("nodeLabels.placement")
        .expect("nodeLabels.placement option");
    let parsed = placement_option
        .parse_value("[H_CENTER, V_TOP, INSIDE]")
        .expect("nodeLabels.placement parsed");
    let placement = parsed
        .downcast_ref::<EnumSet<NodeLabelPlacement>>()
        .expect("EnumSet<NodeLabelPlacement>");
    assert!(placement.contains(&NodeLabelPlacement::HCenter));
    assert!(placement.contains(&NodeLabelPlacement::VTop));
    assert!(placement.contains(&NodeLabelPlacement::Inside));

    let port_labels_option = service
        .get_option_data_by_suffix("portLabels.placement")
        .expect("portLabels.placement option");
    let parsed = port_labels_option
        .parse_value("[OUTSIDE, NEXT_TO_PORT_IF_POSSIBLE]")
        .expect("portLabels.placement parsed");
    let placement = parsed
        .downcast_ref::<EnumSet<PortLabelPlacement>>()
        .expect("EnumSet<PortLabelPlacement>");
    assert!(placement.contains(&PortLabelPlacement::Outside));
    assert!(placement.contains(&PortLabelPlacement::NextToPortIfPossible));

    let edge_routing_option = service
        .get_option_data_by_suffix("edgeRouting")
        .expect("edgeRouting option");
    let parsed = edge_routing_option
        .parse_value("ORTHOGONAL")
        .expect("edgeRouting parsed");
    let edge_routing = parsed
        .downcast_ref::<EdgeRouting>()
        .expect("EdgeRouting");
    assert_eq!(*edge_routing, EdgeRouting::Orthogonal);

    let shape_coords_option = service
        .get_option_data_by_suffix("json.shapeCoords")
        .expect("shapeCoords option");
    let parsed = shape_coords_option
        .parse_value("ROOT")
        .expect("shapeCoords parsed");
    let shape_coords = parsed
        .downcast_ref::<ShapeCoords>()
        .expect("ShapeCoords");
    assert_eq!(*shape_coords, ShapeCoords::Root);

    let edge_coords_option = service
        .get_option_data_by_suffix("json.edgeCoords")
        .expect("edgeCoords option");
    let parsed = edge_coords_option
        .parse_value("PARENT")
        .expect("edgeCoords parsed");
    let edge_coords = parsed
        .downcast_ref::<EdgeCoords>()
        .expect("EdgeCoords");
    assert_eq!(*edge_coords, EdgeCoords::Parent);

    let hierarchy_option = service
        .get_option_data_by_suffix("hierarchyHandling")
        .expect("hierarchyHandling option");
    let parsed = hierarchy_option
        .parse_value("INCLUDE_CHILDREN")
        .expect("hierarchyHandling parsed");
    let handling = parsed
        .downcast_ref::<HierarchyHandling>()
        .expect("HierarchyHandling");
    assert_eq!(*handling, HierarchyHandling::IncludeChildren);

    let content_alignment_option = service
        .get_option_data_by_suffix("contentAlignment")
        .expect("contentAlignment option");
    let parsed = content_alignment_option
        .parse_value("[V_BOTTOM, H_RIGHT]")
        .expect("contentAlignment parsed");
    let alignment = parsed
        .downcast_ref::<EnumSet<ContentAlignment>>()
        .expect("EnumSet<ContentAlignment>");
    assert!(alignment.contains(&ContentAlignment::VBottom));
    assert!(alignment.contains(&ContentAlignment::HRight));
}
