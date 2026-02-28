use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMargin, ElkPadding};
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IndividualSpacings;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::ElkGraphJson;

use super::common::*;

#[test]
fn import_individual_spacings() {
    let graph = r#"
    {
      "id": "n0",
      "children": [
        {
          "id": "outer",
          "layoutOptions": {
              "nodeLabels.padding": "[top=0.0,left=0.0,bottom=0.0,right=0.0]"
          },
          "children": [
            {
              "id": "i1",
              "labels": [
                { "text": "Node 1", "width": 40.0, "height": 15.0 }
              ],
              "layoutOptions": { "nodeLabels.placement": "[H_CENTER, V_TOP, INSIDE]" },
              "width": 60.0,
              "height": 40.0
            },
            {
              "id": "i2",
              "layoutOptions": { "nodeLabels.placement": "[H_CENTER, V_TOP, INSIDE]" },
              "individualSpacings": {
                "nodeLabels.padding": "[top=10.0,left=0.0,bottom=0.0,right=0.0]"
              },
              "labels": [
                { "text": "Node 2", "width": 40.0, "height": 15.0 }
              ],
              "width": 60.0,
              "height": 40.0
            }
          ]
        }
      ]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    let outer = find_node(&node_children(&root), "outer");
    let children = node_children(&outer);
    let i1 = find_node(&children, "i1");
    let i2 = find_node(&children, "i2");

    assert!(!node_has_property(&i1, CoreOptions::SPACING_INDIVIDUAL));
    assert!(node_has_property(&i2, CoreOptions::SPACING_INDIVIDUAL));

    let individual = node_property(&i2, CoreOptions::SPACING_INDIVIDUAL).unwrap();
    assert!(individual
        .properties()
        .has_property(CoreOptions::NODE_LABELS_PADDING));
}

#[test]
fn import_individual_spacings_with_ports_surrounding() {
    let graph = r#"
    {
      "id": "n0",
      "children": [
        {
          "id": "n1",
          "individualSpacings": {
            "spacing.portsSurrounding": "[top=2.0,left=8.0,bottom=6.0,right=4.0]"
          }
        }
      ]
    }
    "#;

    let root = ElkGraphJson::for_graph(graph).to_elk().unwrap();
    let child = find_node(&node_children(&root), "n1");

    let mut individual =
        node_property(&child, CoreOptions::SPACING_INDIVIDUAL).expect("individual spacings");
    let margin = individual
        .properties_mut()
        .get_property(CoreOptions::SPACING_PORTS_SURROUNDING)
        .expect("portsSurrounding");

    assert_eq!(margin, ElkMargin::with_values(2.0, 4.0, 6.0, 8.0));
}

#[test]
fn export_individual_spacings() {
    let graph = ElkGraphUtil::create_graph();
    set_node_property(&graph, CoreOptions::SPACING_NODE_NODE, 10.0);

    let mut individual = IndividualSpacings::new();
    individual
        .properties_mut()
        .set_property(CoreOptions::SPACING_NODE_NODE, Some(20.0));
    set_node_property(&graph, CoreOptions::SPACING_INDIVIDUAL, individual);

    let json = ElkGraphJson::for_elk(graph)
        .omit_unknown_layout_options(true)
        .to_json();

    assert!(json.contains("individualSpacings"));
    assert!(json.contains("nodeNode"));
    assert!(json.contains("10"));
    assert!(json.contains("20"));
}

#[test]
fn export_individual_spacings_with_padding() {
    let graph = ElkGraphUtil::create_graph();

    let mut individual = IndividualSpacings::new();
    individual.properties_mut().set_property(
        CoreOptions::NODE_LABELS_PADDING,
        Some(ElkPadding::with_values(1.0, 2.0, 3.0, 4.0)),
    );
    set_node_property(&graph, CoreOptions::SPACING_INDIVIDUAL, individual);

    let json = ElkGraphJson::for_elk(graph)
        .omit_unknown_layout_options(true)
        .to_json();

    assert!(json.contains("individualSpacings"));
    assert!(json.contains("elk.nodeLabels.padding"));
    assert!(json.contains("[top=1,left=4,bottom=3,right=2]"));
}

#[test]
fn export_individual_spacings_with_ports_surrounding() {
    let graph = ElkGraphUtil::create_graph();

    let mut individual = IndividualSpacings::new();
    individual.properties_mut().set_property(
        CoreOptions::SPACING_PORTS_SURROUNDING,
        Some(ElkMargin::with_values(2.0, 4.0, 6.0, 8.0)),
    );
    set_node_property(&graph, CoreOptions::SPACING_INDIVIDUAL, individual);

    let json = ElkGraphJson::for_elk(graph)
        .omit_unknown_layout_options(true)
        .to_json();

    assert!(json.contains("individualSpacings"));
    assert!(json.contains("elk.spacing.portsSurrounding"));
    assert!(json.contains("[top=2,left=8,bottom=6,right=4]"));
}

#[test]
fn export_no_individual_spacings() {
    let graph = ElkGraphUtil::create_graph();
    let json = ElkGraphJson::for_elk(graph)
        .omit_unknown_layout_options(true)
        .to_json();
    assert!(!json.contains("individualSpacings"));
    assert!(!json.contains("IndividualSpacings"));
}
