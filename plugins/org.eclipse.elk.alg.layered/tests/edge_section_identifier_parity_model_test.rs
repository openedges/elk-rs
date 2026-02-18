mod elkt_test_loader;
mod issue_support;

use std::path::PathBuf;

use elkt_test_loader::{find_edge_by_identifier, load_layered_graph_from_elkt};
use issue_support::{init_layered_options, run_recursive_layout};

#[test]
fn preserves_issue_476_horizontal_section_identifier() {
    init_layered_options();
    assert_first_section_identifier(
        "476_multiLabelInHorizontalLayout.elkt",
        "Node1",
        "Node2",
        "ES1",
    );
}

#[test]
fn preserves_issue_476_vertical_section_identifier() {
    init_layered_options();
    assert_first_section_identifier(
        "476_multiLabelInVerticalLayout.elkt",
        "Node1",
        "Node2",
        "ES1",
    );
}

fn assert_first_section_identifier(
    model_name: &str,
    source_identifier: &str,
    target_identifier: &str,
    expected_section_id: &str,
) {
    let resource = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../external/elk-models/tickets/layered/{model_name}"
    ));
    if !resource.exists() {
        eprintln!(
            "edge section identifier resource missing, skipping: {}",
            resource.display()
        );
        return;
    }

    let path = resource.to_string_lossy();
    let graph = load_layered_graph_from_elkt(path.as_ref())
        .unwrap_or_else(|err| panic!("{model_name} should load: {err}"));
    run_recursive_layout(&graph);

    let edge = find_edge_by_identifier(&graph, source_identifier, target_identifier)
        .unwrap_or_else(|| panic!("edge {source_identifier}->{target_identifier} should exist"));

    let actual_section_id = {
        let mut edge_mut = edge.borrow_mut();
        let section = edge_mut
            .sections()
            .get(0)
            .expect("edge should have at least one section");
        let identifier = section.borrow().identifier().map(ToString::to_string);
        identifier
    };

    assert_eq!(
        actual_section_id.as_deref(),
        Some(expected_section_id),
        "section id should be preserved after layout for {model_name}"
    );
}
