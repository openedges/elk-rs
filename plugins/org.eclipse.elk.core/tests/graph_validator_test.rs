use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutAlgorithmData, LayoutAlgorithmResolver, LayoutMetaDataRegistry,
    LayoutMetaDataService,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, IGraphElementVisitor};
use org_eclipse_elk_core::org::eclipse::elk::core::validation::{
    GraphIssue, GraphValidator, IValidatingGraphElementVisitor, Severity,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;
use org_eclipse_elk_graph::org::eclipse::elk::graph::{
    ElkConnectableShapeRef, ElkEdge, ElkEdgeRef, ElkEdgeSection, ElkEdgeSectionRef,
    ElkGraphElementRef,
};

#[test]
fn graph_validator_unconnected_edge() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    ElkGraphUtil::create_edge(Some(graph.clone()));

    let mut validator = GraphValidator::new();
    ElkUtil::apply_visitors(&graph, &mut [&mut validator]);

    let issues = validator.issues().expect("issues");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity(), Severity::Error);
    assert_eq!(issues[0].message(), "Edge is not connected.");
}

#[test]
fn graph_validator_custom_validation() {
    LayoutMetaDataService::get_instance();
    let provider = FooProvider;
    LayoutMetaDataService::get_instance().register_layout_meta_data_provider(&provider);

    let graph = ElkGraphUtil::create_graph();
    let node = ElkGraphUtil::create_node(Some(graph.clone()));
    {
        let mut node_mut = node.borrow_mut();
        node_mut
            .connectable()
            .shape()
            .graph_element()
            .set_identifier(Some("foo".to_string()));
    }
    {
        let mut graph_mut = graph.borrow_mut();
        graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(
                CoreOptions::ALGORITHM,
                Some("test.foo.algorithm".to_string()),
            );
    }

    let mut resolver = LayoutAlgorithmResolver::new();
    let mut validator = GraphValidator::new();
    ElkUtil::apply_visitors(&graph, &mut [&mut resolver, &mut validator]);

    let issues = validator.issues().expect("issues");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity(), Severity::Error);
    assert_eq!(issues[0].message(), "FOO!");
}

#[test]
fn graph_validator_edge_containment_warning() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let n2 = ElkGraphUtil::create_node(Some(graph.clone()));
    let edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );

    ElkEdge::set_containing_node(&edge, Some(n1.clone()));

    let mut validator = GraphValidator::new();
    ElkUtil::apply_visitors(&graph, &mut [&mut validator]);

    let issues = validator.issues().expect("issues");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity(), Severity::Warning);
    assert!(issues[0].message().contains("Edge should be contained in"));
}

#[test]
fn graph_validator_section_incoming_shape_not_source() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let n2 = ElkGraphUtil::create_node(Some(graph.clone()));
    let edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );

    let section = create_section(&edge);
    section
        .borrow_mut()
        .set_incoming_shape(Some(ElkConnectableShapeRef::Node(n2.clone())));

    let mut validator = GraphValidator::new();
    ElkUtil::apply_visitors(&graph, &mut [&mut validator]);

    let issues = validator.issues().expect("issues");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity(), Severity::Error);
    assert!(issues[0]
        .message()
        .contains("incoming shape is not a source"));
}

#[test]
fn graph_validator_section_outgoing_shape_not_target() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let n2 = ElkGraphUtil::create_node(Some(graph.clone()));
    let edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );

    let section = create_section(&edge);
    section
        .borrow_mut()
        .set_outgoing_shape(Some(ElkConnectableShapeRef::Node(n1.clone())));

    let mut validator = GraphValidator::new();
    ElkUtil::apply_visitors(&graph, &mut [&mut validator]);

    let issues = validator.issues().expect("issues");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity(), Severity::Error);
    assert!(issues[0]
        .message()
        .contains("outgoing shape is not a target"));
}

#[test]
fn graph_validator_section_incoming_conflict() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let n1 = ElkGraphUtil::create_node(Some(graph.clone()));
    let n2 = ElkGraphUtil::create_node(Some(graph.clone()));
    let edge = ElkGraphUtil::create_simple_edge(
        ElkConnectableShapeRef::Node(n1.clone()),
        ElkConnectableShapeRef::Node(n2.clone()),
    );

    let section = create_section(&edge);
    let other = create_section(&edge);
    section
        .borrow_mut()
        .set_incoming_shape(Some(ElkConnectableShapeRef::Node(n1.clone())));
    section.borrow_mut().set_incoming_sections(vec![other]);

    let mut validator = GraphValidator::new();
    ElkUtil::apply_visitors(&graph, &mut [&mut validator]);

    let issues = validator.issues().expect("issues");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity(), Severity::Error);
    assert!(issues[0]
        .message()
        .contains("cannot be connected to an ElkNode"));
}

struct FooProvider;

impl ILayoutMetaDataProvider for FooProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        let factory: Arc<dyn Fn() -> Box<dyn IValidatingGraphElementVisitor> + Send + Sync> =
            Arc::new(|| Box::new(FooValidator::new()));
        let mut data = LayoutAlgorithmData::new("test.foo.algorithm");
        data.set_validator_factory(Some(factory));
        registry.register_algorithm(data);
    }
}

struct FooValidator {
    issues: Vec<GraphIssue>,
}

impl FooValidator {
    fn new() -> Self {
        FooValidator { issues: Vec::new() }
    }
}

impl IGraphElementVisitor for FooValidator {
    fn visit(&mut self, element: &ElkGraphElementRef) {
        let is_match = match element {
            ElkGraphElementRef::Node(node) => {
                let mut node_mut = node.borrow_mut();
                let identifier = node_mut.connectable().shape().graph_element().identifier();
                matches!(identifier, Some("foo"))
            }
            _ => false,
        };

        if is_match {
            self.issues.push(GraphIssue::new(
                Some(element.clone()),
                "FOO!",
                Severity::Error,
            ));
        }
    }

    fn issues(&self) -> Option<&[GraphIssue]> {
        Some(&self.issues)
    }
}

fn create_section(edge: &ElkEdgeRef) -> ElkEdgeSectionRef {
    let section = ElkEdgeSection::new();
    edge.borrow_mut().sections().add(section.clone());
    section
}
