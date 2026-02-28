use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{ElkUtil, IGraphElementVisitor};
use org_eclipse_elk_core::org::eclipse::elk::core::validation::{LayoutOptionValidator, Severity};
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkGraphUtil;

#[test]
fn layout_option_validator_invalid_type() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();
    let option_data = LayoutMetaDataService::get_instance()
        .get_option_data(CoreOptions::ASPECT_RATIO.id())
        .expect("aspect ratio option data");

    {
        let mut graph_mut = graph.borrow_mut();
        graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property_any(option_data.id(), Some(Arc::new("foo".to_string())));
    }

    let mut validator = LayoutOptionValidator::new();
    ElkUtil::apply_visitors(&graph, &mut [&mut validator]);

    let issues = validator.issues().expect("issues");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity(), Severity::Error);
    assert_eq!(
        issues[0].message(),
        "The assigned value foo of the option 'Aspect Ratio' does not match the type Double."
    );
}

#[test]
fn layout_option_validator_exclusive_lower_bound() {
    LayoutMetaDataService::get_instance();
    let graph = ElkGraphUtil::create_graph();

    {
        let mut graph_mut = graph.borrow_mut();
        graph_mut
            .connectable()
            .shape()
            .graph_element()
            .properties_mut()
            .set_property(CoreOptions::ASPECT_RATIO, Some(0.0));
    }

    let mut validator = LayoutOptionValidator::new();
    ElkUtil::apply_visitors(&graph, &mut [&mut validator]);

    let issues = validator.issues().expect("issues");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity(), Severity::Error);
    assert_eq!(
        issues[0].message(),
        "The assigned value 0.0 of the option 'Aspect Ratio' is less than the lower bound 0.0 (exclusive)."
    );
}
