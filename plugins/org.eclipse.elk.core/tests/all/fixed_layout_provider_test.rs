use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkPadding, KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::FixedLayouterOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    FixedLayoutProvider, NullElkProgressMonitor,
};
use org_eclipse_elk_core::org::eclipse::elk::core::IGraphLayoutEngine;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkGraphElementRef, ElkNodeRef};

fn node_bounds(node: &ElkNodeRef) -> (f64, f64, f64, f64) {
    let mut node_mut = node.borrow_mut();
    let shape = node_mut.connectable().shape();
    (shape.x(), shape.y(), shape.width(), shape.height())
}

#[test]
fn fixed_layout_applies_positions_and_sizes() {
    LayoutMetaDataService::get_instance();

    let root = ElkGraphUtil::create_graph();
    let child = ElkGraphUtil::create_node(Some(root.clone()));

    {
        let mut child_mut = child.borrow_mut();
        child_mut.connectable().shape().set_dimensions(30.0, 40.0);
        child_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(
                FixedLayouterOptions::POSITION,
                Some(KVector::with_values(10.0, 20.0)),
            );
    }

    let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Node(child.clone())));
    {
        let mut label_mut = label.borrow_mut();
        label_mut.shape().set_dimensions(5.0, 5.0);
        label_mut
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(
                FixedLayouterOptions::POSITION,
                Some(KVector::with_values(2.0, 3.0)),
            );
    }

    {
        let mut root_mut = root.borrow_mut();
        root_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(FixedLayouterOptions::PADDING, Some(ElkPadding::new()));
    }

    let mut provider = FixedLayoutProvider::new();
    let mut monitor = NullElkProgressMonitor;
    provider.layout(&root, &mut monitor);

    let (child_x, child_y, child_w, child_h) = node_bounds(&child);
    assert_eq!(child_x, 10.0);
    assert_eq!(child_y, 20.0);

    let (root_x, root_y, root_w, root_h) = node_bounds(&root);
    assert_eq!(root_x, 0.0);
    assert_eq!(root_y, 0.0);
    assert!(root_w >= child_x + child_w);
    assert!(root_h >= child_y + child_h);
}
