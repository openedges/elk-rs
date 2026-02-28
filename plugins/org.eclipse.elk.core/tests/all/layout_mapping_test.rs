use std::rc::Rc;

use org_eclipse_elk_core::org::eclipse::elk::core::service::LayoutMapping;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkGraphElementRef;

#[test]
fn layout_mapping_tracks_elements() {
    let workbench = Rc::new("workbench".to_string());
    let mut mapping = LayoutMapping::new(Some(workbench.clone()));

    let node = ElkGraphUtil::create_node(None);
    mapping.set_layout_graph(node.clone());

    let parent = Rc::new("parent".to_string());
    mapping.set_parent_element(Some(parent.clone()));

    let diagram: Rc<dyn std::any::Any> = Rc::new("diagram".to_string());
    let element = ElkGraphElementRef::Node(node.clone());
    mapping.insert_mapping(element.clone(), diagram.clone());

    let diagram_any = mapping.diagram_for(&element).expect("diagram mapping");
    let diagram_str = diagram_any.as_ref().downcast_ref::<String>().unwrap();
    assert_eq!(diagram_str, "diagram");

    let element_back = mapping.element_for(&diagram).expect("element mapping");
    match element_back {
        ElkGraphElementRef::Node(found) => assert!(std::rc::Rc::ptr_eq(&found, &node)),
        _ => panic!("unexpected element type"),
    }

    let workbench_any = mapping.workbench_part().expect("workbench");
    let workbench_str = workbench_any.as_ref().downcast_ref::<String>().unwrap();
    assert_eq!(workbench_str, "workbench");

    let parent_any = mapping.parent_element().expect("parent");
    let parent_str = parent_any.as_ref().downcast_ref::<String>().unwrap();
    assert_eq!(parent_str, "parent");
}
