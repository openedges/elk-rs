use wasm_bindgen::prelude::*;

use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api;

/// Run ELK layout on a JSON graph string.
///
/// - `graph_json`: ELK graph in JSON format
/// - `options_json`: global layout options as JSON object (merged as defaults)
///
/// Returns the laid-out graph as a JSON string.
#[wasm_bindgen]
pub fn layout_json(graph_json: &str, options_json: &str) -> Result<String, JsError> {
    layout_api::layout_json(graph_json, options_json).map_err(|e| JsError::new(&e))
}

/// Return all registered layout algorithms as a JSON array string.
#[wasm_bindgen]
pub fn known_layout_algorithms() -> Result<String, JsError> {
    layout_api::known_layout_algorithms().map_err(|e| JsError::new(&e))
}

/// Return all registered layout options as a JSON array string.
#[wasm_bindgen]
pub fn known_layout_options() -> Result<String, JsError> {
    layout_api::known_layout_options().map_err(|e| JsError::new(&e))
}

/// Return all registered layout categories as a JSON array string.
#[wasm_bindgen]
pub fn known_layout_categories() -> Result<String, JsError> {
    layout_api::known_layout_categories().map_err(|e| JsError::new(&e))
}
