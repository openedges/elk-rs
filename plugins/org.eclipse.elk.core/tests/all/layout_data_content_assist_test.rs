use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    LayoutDataContentAssist, LayoutMetaDataService, LayoutOptionData,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    BoxLayouterOptions, CoreOptions, FixedLayouterOptions,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::MapPropertyHolder;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{ElkGraphElementRef, ElkNodeRef};

enum ElementKind {
    Node,
    Edge,
    Port,
    Label,
}

fn init_metadata() {
    LayoutMetaDataService::get_instance();
}

fn option_data(id: &str) -> LayoutOptionData {
    LayoutMetaDataService::get_instance()
        .get_option_data(id)
        .expect("Expected option data to be registered.")
}

fn make_element(kind: ElementKind) -> ElkGraphElementRef {
    match kind {
        ElementKind::Node => {
            let node = ElkGraphUtil::create_graph();
            ElkGraphElementRef::Node(node)
        }
        ElementKind::Edge => {
            let edge = ElkGraphUtil::create_edge(None);
            ElkGraphElementRef::Edge(edge)
        }
        ElementKind::Port => {
            let root = ElkGraphUtil::create_graph();
            let port = ElkGraphUtil::create_port(Some(root));
            ElkGraphElementRef::Port(port)
        }
        ElementKind::Label => {
            let root = ElkGraphUtil::create_graph();
            let label = ElkGraphUtil::create_label(Some(ElkGraphElementRef::Node(root)));
            ElkGraphElementRef::Label(label)
        }
    }
}

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

#[test]
fn layout_option_prefixes() {
    init_metadata();

    struct Case {
        kind: ElementKind,
        expected_id: &'static str,
        prefix: &'static str,
    }

    let cases = vec![
        Case {
            kind: ElementKind::Node,
            expected_id: CoreOptions::PORT_CONSTRAINTS.id(),
            prefix: "org.eclipse.elk.portConstraints",
        },
        Case {
            kind: ElementKind::Node,
            expected_id: CoreOptions::PORT_CONSTRAINTS.id(),
            prefix: "eclipse.elk.portConstraints",
        },
        Case {
            kind: ElementKind::Node,
            expected_id: CoreOptions::PORT_CONSTRAINTS.id(),
            prefix: "elk.portConstraints",
        },
        Case {
            kind: ElementKind::Node,
            expected_id: CoreOptions::PORT_CONSTRAINTS.id(),
            prefix: "portConstraints",
        },
        Case {
            kind: ElementKind::Node,
            expected_id: CoreOptions::PORT_CONSTRAINTS.id(),
            prefix: "portCo",
        },
        Case {
            kind: ElementKind::Node,
            expected_id: CoreOptions::PORT_CONSTRAINTS.id(),
            prefix: "rt Constr",
        },
        Case {
            kind: ElementKind::Edge,
            expected_id: CoreOptions::PRIORITY.id(),
            prefix: "prio",
        },
        Case {
            kind: ElementKind::Port,
            expected_id: CoreOptions::PORT_SIDE.id(),
            prefix: "port.sid",
        },
        Case {
            kind: ElementKind::Label,
            expected_id: CoreOptions::NODE_LABELS_PLACEMENT.id(),
            prefix: "nodeLabels.pla",
        },
        Case {
            kind: ElementKind::Node,
            expected_id: CoreOptions::PADDING.id(),
            prefix: "",
        },
        Case {
            kind: ElementKind::Node,
            expected_id: CoreOptions::ALIGNMENT.id(),
            prefix: "",
        },
    ];

    for case in cases {
        let element = make_element(case.kind);
        let expected = option_data(case.expected_id);
        let proposals = LayoutDataContentAssist::get_layout_option_proposals(&element, case.prefix);
        let contains = proposals.iter().any(|p| {
            p.data
                .as_ref()
                .map(|data| data.id() == expected.id())
                .unwrap_or(false)
        });
        assert!(
            contains,
            "Expected option '{}' for prefix '{}'.",
            expected.id(),
            case.prefix
        );
    }
}

#[test]
fn layout_option_parent_nodes() {
    init_metadata();
    let root = ElkGraphUtil::create_graph();
    let child = ElkGraphUtil::create_node(Some(root.clone()));

    let root_props = LayoutDataContentAssist::get_layout_option_proposals(
        &ElkGraphElementRef::Node(root),
        CoreOptions::ALGORITHM.id(),
    );
    let root_contains = root_props.iter().any(|p| {
        p.data
            .as_ref()
            .map(|data| data.id() == CoreOptions::ALGORITHM.id())
            .unwrap_or(false)
    });
    assert!(root_contains);

    let child_props = LayoutDataContentAssist::get_layout_option_proposals(
        &ElkGraphElementRef::Node(child),
        CoreOptions::ALGORITHM.id(),
    );
    let child_contains = child_props.iter().any(|p| {
        p.data
            .as_ref()
            .map(|data| data.id() == CoreOptions::ALGORITHM.id())
            .unwrap_or(false)
    });
    assert!(!child_contains);
}

#[test]
fn layout_option_algorithm_specific_known() {
    init_metadata();
    let root = ElkGraphUtil::create_graph();
    ElkGraphUtil::create_node(Some(root.clone()));
    with_node_properties_mut(&root, |props| {
        props.set_property(
            CoreOptions::ALGORITHM,
            Some(BoxLayouterOptions::ALGORITHM_ID.to_string()),
        );
    });

    let proposals = LayoutDataContentAssist::get_layout_option_proposals(
        &ElkGraphElementRef::Node(root),
        CoreOptions::EXPAND_NODES.id(),
    );
    let contains = proposals.iter().any(|p| {
        p.data
            .as_ref()
            .map(|data| data.id() == CoreOptions::EXPAND_NODES.id())
            .unwrap_or(false)
    });
    assert!(contains);
}

#[test]
fn layout_option_algorithm_specific_unknown() {
    init_metadata();
    let root = ElkGraphUtil::create_graph();
    ElkGraphUtil::create_node(Some(root.clone()));
    with_node_properties_mut(&root, |props| {
        props.set_property(
            CoreOptions::ALGORITHM,
            Some(FixedLayouterOptions::ALGORITHM_ID.to_string()),
        );
    });

    let proposals = LayoutDataContentAssist::get_layout_option_proposals(
        &ElkGraphElementRef::Node(root),
        CoreOptions::EXPAND_NODES.id(),
    );
    let contains = proposals.iter().any(|p| {
        p.data
            .as_ref()
            .map(|data| data.id() == CoreOptions::EXPAND_NODES.id())
            .unwrap_or(false)
    });
    assert!(!contains);
}

#[test]
fn layout_option_group_is_added() {
    init_metadata();
    let root = ElkGraphUtil::create_graph();
    ElkGraphUtil::create_node(Some(root.clone()));
    let option = option_data(CoreOptions::SPACING_NODE_NODE.id());
    let proposals = LayoutDataContentAssist::get_layout_option_proposals(
        &ElkGraphElementRef::Node(root),
        "nodeN",
    );
    let contains = proposals.iter().any(|p| {
        p.data
            .as_ref()
            .map(|data| data.id() == option.id() && p.proposal.contains(option.group()))
            .unwrap_or(false)
    });
    assert!(contains);
}
