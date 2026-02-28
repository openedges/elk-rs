use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::{ElkGraphJson, JsonImportError};

fn assert_import_error(input: &str) {
    assert!(matches!(
        ElkGraphJson::for_graph(input).to_elk(),
        Err(JsonImportError::Import(_))
    ));
}

#[test]
fn test_no_id() {
    assert_import_error("{}");
}

#[test]
fn test_wrong_id_type_number() {
    assert_import_error("{ id: 1.2 }");
}

#[test]
fn test_wrong_id_type_object() {
    assert_import_error("{ id: {} }");
}

#[test]
fn test_wrong_id_type_array() {
    assert_import_error("{ id: [] }");
}

#[test]
fn test_wrong_id_type_boolean() {
    assert_import_error("{ id: true }");
}

#[test]
fn test_good_id_string() {
    ElkGraphJson::for_graph("{ id: 'foo' }").to_elk().unwrap();
}

#[test]
fn test_good_id_int() {
    ElkGraphJson::for_graph("{ id: 3 }").to_elk().unwrap();
}
