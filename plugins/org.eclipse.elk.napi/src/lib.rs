#[macro_use]
extern crate napi_derive;

use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api;

/// Run ELK layout on a JSON graph string.
///
/// - `graph_json`: ELK graph in JSON format
/// - `options_json`: global layout options as JSON object (merged as defaults)
///
/// Returns the laid-out graph as a JSON string.
#[napi(js_name = "layout_json")]
pub fn layout_json(graph_json: String, options_json: String) -> napi::Result<String> {
    layout_api::layout_json(&graph_json, &options_json)
        .map_err(napi::Error::from_reason)
}

/// Return all registered layout algorithms as a JSON array string.
#[napi(js_name = "known_layout_algorithms")]
pub fn known_layout_algorithms() -> napi::Result<String> {
    layout_api::known_layout_algorithms()
        .map_err(napi::Error::from_reason)
}

/// Return all registered layout options as a JSON array string.
#[napi(js_name = "known_layout_options")]
pub fn known_layout_options() -> napi::Result<String> {
    layout_api::known_layout_options()
        .map_err(napi::Error::from_reason)
}

/// Return all registered layout categories as a JSON array string.
#[napi(js_name = "known_layout_categories")]
pub fn known_layout_categories() -> napi::Result<String> {
    layout_api::known_layout_categories()
        .map_err(napi::Error::from_reason)
}
