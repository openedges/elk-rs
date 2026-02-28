use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    LayoutAlgorithmResolver, LayoutMetaDataService,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::ElkUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

fn with_node_properties_mut<R>(
    node: &ElkNodeRef,
    f: impl FnOnce(&mut MapPropertyHolder) -> R,
) -> R {
    let mut node_mut = node.borrow_mut();
    let props = node_mut
        .connectable()
        .shape()
        .graph_element()
        .properties_mut();
    f(props)
}

fn resolved_algorithm_id(node: &ElkNodeRef) -> Option<String> {
    with_node_properties_mut(node, |props| {
        props
            .get_property(CoreOptions::RESOLVED_ALGORITHM)
            .map(|value| value.id().to_string())
    })
}

#[test]
fn resolver_sets_default_for_hierarchical_node() {
    LayoutMetaDataService::get_instance();
    let root = ElkGraphUtil::create_graph();
    let _child = ElkGraphUtil::create_node(Some(root.clone()));

    let mut resolver = LayoutAlgorithmResolver::new();
    ElkUtil::apply_visitors(&root, &mut [&mut resolver]);

    let resolved = resolved_algorithm_id(&root).expect("resolved algorithm");
    assert_eq!(resolved, "org.eclipse.elk.layered");
    assert!(resolver.errors().is_empty());
}

#[test]
fn resolver_skips_leaf_node_without_inside_self_loops() {
    LayoutMetaDataService::get_instance();
    let root = ElkGraphUtil::create_graph();

    let mut resolver = LayoutAlgorithmResolver::new();
    ElkUtil::apply_visitors(&root, &mut [&mut resolver]);

    let resolved = resolved_algorithm_id(&root);
    assert!(resolved.is_none());
    assert!(resolver.errors().is_empty());
}

#[test]
fn resolver_sets_default_for_inside_self_loops() {
    LayoutMetaDataService::get_instance();
    let root = ElkGraphUtil::create_graph();
    with_node_properties_mut(&root, |props| {
        props.set_property(CoreOptions::INSIDE_SELF_LOOPS_ACTIVATE, Some(true));
    });

    let mut resolver = LayoutAlgorithmResolver::new();
    ElkUtil::apply_visitors(&root, &mut [&mut resolver]);

    let resolved = resolved_algorithm_id(&root).expect("resolved algorithm");
    assert_eq!(resolved, "org.eclipse.elk.layered");
    assert!(resolver.errors().is_empty());
}

#[test]
fn resolver_records_error_for_unknown_algorithm() {
    LayoutMetaDataService::get_instance();
    let root = ElkGraphUtil::create_graph();
    let _child = ElkGraphUtil::create_node(Some(root.clone()));
    with_node_properties_mut(&root, |props| {
        props.set_property(CoreOptions::ALGORITHM, Some("unknown.alg".to_string()));
    });

    let mut resolver = LayoutAlgorithmResolver::new();
    ElkUtil::apply_visitors(&root, &mut [&mut resolver]);

    let errors = resolver.errors();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message().contains("unknown.alg"));
}

#[test]
fn resolver_accepts_suffix_algorithm_id() {
    LayoutMetaDataService::get_instance();
    let root = ElkGraphUtil::create_graph();
    let _child = ElkGraphUtil::create_node(Some(root.clone()));
    with_node_properties_mut(&root, |props| {
        props.set_property(CoreOptions::ALGORITHM, Some("layered".to_string()));
    });

    let mut resolver = LayoutAlgorithmResolver::new();
    ElkUtil::apply_visitors(&root, &mut [&mut resolver]);

    let resolved = resolved_algorithm_id(&root).expect("resolved algorithm");
    assert_eq!(resolved, "org.eclipse.elk.layered");
    assert!(resolver.errors().is_empty());
}
