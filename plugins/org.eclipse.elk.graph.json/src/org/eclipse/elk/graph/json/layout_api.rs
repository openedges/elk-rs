//! Public layout API for WASM and NAPI bindings.
//!
//! Provides the core layout pipeline functions shared by both wasm-bindgen
//! and napi-rs wrappers.

use std::any::Any;
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use serde_json::{json, Map, Value};

use org_eclipse_elk_alg_disco::org::eclipse::elk::alg::disco::options::disco_meta_data_provider::DisCoMetaDataProvider;
use org_eclipse_elk_alg_disco::org::eclipse::elk::alg::disco::DisCoLayoutProvider;
use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::force_layout_provider::ForceLayoutProvider;
use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::options::force_meta_data_provider::ForceMetaDataProvider;
use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::options::stress_meta_data_provider::StressMetaDataProvider;
use org_eclipse_elk_alg_force::org::eclipse::elk::alg::force::stress::stress_layout_provider::StressLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::elk_layered::ElkLayered;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::layered_layout_provider::LayeredLayoutProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredMetaDataProvider;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_alg_mrtree::org::eclipse::elk::alg::mrtree::options::mrtree_meta_data_provider::MrTreeMetaDataProvider;
use org_eclipse_elk_alg_mrtree::org::eclipse::elk::alg::mrtree::tree_layout_provider::TreeLayoutProvider;
use org_eclipse_elk_alg_radial::org::eclipse::elk::alg::radial::options::radial_meta_data_provider::RadialMetaDataProvider;
use org_eclipse_elk_alg_radial::org::eclipse::elk::alg::radial::radial_layout_provider::RadialLayoutProvider;
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::options::rect_packing_meta_data_provider::RectPackingMetaDataProvider;
use org_eclipse_elk_alg_rectpacking::org::eclipse::elk::alg::rectpacking::rect_packing_layout_provider::RectPackingLayoutProvider;
use org_eclipse_elk_alg_spore::org::eclipse::elk::alg::spore::options::spore_meta_data_provider::SporeMetaDataProvider;
use org_eclipse_elk_alg_spore::org::eclipse::elk::alg::spore::overlap_removal_layout_provider::OverlapRemovalLayoutProvider;
use org_eclipse_elk_alg_spore::org::eclipse::elk::alg::spore::shrink_tree_layout_provider::ShrinkTreeLayoutProvider;
use org_eclipse_elk_alg_vertiflex::org::eclipse::elk::alg::vertiflex::options::vertiflex_meta_data_provider::VertiFlexMetaDataProvider;
use org_eclipse_elk_alg_vertiflex::org::eclipse::elk::alg::vertiflex::vertiflex_layout_provider::VertiFlexLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::abstract_layout_provider::AbstractLayoutProvider;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::graph_layout_engine::IGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::recursive_graph_layout_engine::RecursiveGraphLayoutEngine;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    AlgorithmFactory, BasicProgressMonitor, InstancePool, Maybe,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::GraphFeature;

use crate::org::eclipse::elk::graph::json::{ElkGraphJson, JsonImporter};

fn provider_pool<F>(creator: F) -> Arc<InstancePool<Box<dyn AbstractLayoutProvider>>>
where
    F: Fn() -> Box<dyn AbstractLayoutProvider> + Send + Sync + 'static,
{
    Arc::new(InstancePool::new(Box::new(AlgorithmFactory::new(creator))))
}

/// Initialize all layout algorithms and metadata providers.
/// Safe to call multiple times; initialization happens only once.
pub fn ensure_initialized() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        initialize_plain_java_layout();

        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
        service.register_layout_meta_data_provider(&DisCoMetaDataProvider);
        service.register_layout_meta_data_provider(&ForceMetaDataProvider);
        service.register_layout_meta_data_provider(&MrTreeMetaDataProvider);
        service.register_layout_meta_data_provider(&RadialMetaDataProvider);
        service.register_layout_meta_data_provider(&RectPackingMetaDataProvider);
        service.register_layout_meta_data_provider(&SporeMetaDataProvider);
        service.register_layout_meta_data_provider(&StressMetaDataProvider);
        service.register_layout_meta_data_provider(&VertiFlexMetaDataProvider);

        service.override_algorithm_provider_pool(
            "org.eclipse.elk.layered",
            provider_pool(|| Box::new(LayeredLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.disco",
            provider_pool(|| Box::new(DisCoLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.force",
            provider_pool(|| Box::new(ForceLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.stress",
            provider_pool(|| Box::new(StressLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.mrtree",
            provider_pool(|| Box::new(TreeLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.radial",
            provider_pool(|| Box::new(RadialLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.rectpacking",
            provider_pool(|| Box::new(RectPackingLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.sporeOverlap",
            provider_pool(|| Box::new(OverlapRemovalLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.sporeCompaction",
            provider_pool(|| Box::new(ShrinkTreeLayoutProvider::new())),
        );
        service.override_algorithm_provider_pool(
            "org.eclipse.elk.vertiflex",
            provider_pool(|| Box::new(VertiFlexLayoutProvider::new())),
        );
    });
}

fn panic_payload_to_string(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "unknown panic payload".to_string()
}

/// Merge global layout options into every element of the graph JSON as defaults.
///
/// Global options are applied recursively to the root and all descendant
/// elements (children, edges, ports, labels). For each element, a global
/// option is only added if no existing option with the same canonical ID
/// is already present. This correctly handles option name aliases
/// (e.g. `elk.direction` vs `org.eclipse.elk.direction`) by resolving
/// all keys to canonical IDs via the LayoutMetaDataService.
fn merge_global_options(root_value: &mut Value, options_json: &str) -> Result<(), String> {
    if options_json.is_empty() {
        return Ok(());
    }

    let global: Value = serde_json::from_str(options_json)
        .map_err(|e| format!("Failed to parse options JSON: {e}"))?;
    let Some(global_opts) = global.as_object() else {
        return Ok(());
    };
    if global_opts.is_empty() {
        return Ok(());
    }

    apply_global_options_recursive(root_value, global_opts);
    Ok(())
}

/// Recursively apply global layout options to an element and all its
/// descendants (children, edges, ports, labels).
fn apply_global_options_recursive(element: &mut Value, global_opts: &Map<String, Value>) {
    if !element.is_object() {
        return;
    }

    // Step 1: merge global options into this element's layoutOptions/properties
    {
        let obj = element.as_object_mut().unwrap();
        let field = if obj.contains_key("layoutOptions") {
            "layoutOptions"
        } else if obj.contains_key("properties") {
            "properties"
        } else {
            "layoutOptions"
        };

        let service = LayoutMetaDataService::get_instance();
        let mut existing_canonical_ids = std::collections::HashSet::new();
        if let Some(Value::Object(existing)) = obj.get(field) {
            for key in existing.keys() {
                if let Some(opt) = service.get_option_data_by_suffix(key) {
                    existing_canonical_ids.insert(opt.id().to_string());
                } else {
                    existing_canonical_ids.insert(key.clone());
                }
            }
        }

        let layout_options = obj
            .entry(field)
            .or_insert_with(|| Value::Object(Map::new()));
        if let Some(lo) = layout_options.as_object_mut() {
            for (key, value) in global_opts {
                let canonical = service
                    .get_option_data_by_suffix(key)
                    .map(|opt| opt.id().to_string())
                    .unwrap_or_else(|| key.clone());
                if !existing_canonical_ids.contains(&canonical) {
                    lo.insert(key.clone(), value.clone());
                    existing_canonical_ids.insert(canonical);
                }
            }
        }
    }

    // Step 2: recurse into sub-elements (children, edges, ports, labels)
    for arr_key in &["children", "edges", "ports", "labels"] {
        if let Some(arr) = element.get_mut(*arr_key) {
            if let Some(items) = arr.as_array_mut() {
                for item in items {
                    apply_global_options_recursive(item, global_opts);
                }
            }
        }
    }
}

/// Run ELK layout on a JSON graph, returning the laid-out graph as JSON.
///
/// - `graph_json`: ELK graph in JSON format (same as elkjs input)
/// - `options_json`: global layout options as JSON object (merged as defaults)
///
/// Returns the laid-out graph JSON string, or an error message.
pub fn layout_json(graph_json: &str, options_json: &str) -> Result<String, String> {
    ensure_initialized();

    let result = panic::catch_unwind(AssertUnwindSafe(|| -> Result<String, String> {
        let mut input_value: Value = serde_json::from_str(graph_json)
            .map_err(|err| format!("Failed to parse graph JSON: {err}"))?;

        merge_global_options(&mut input_value, options_json)?;

        let shared = Rc::new(RefCell::new(input_value));
        let mut importer_slot: Maybe<JsonImporter> = Maybe::default();
        let root = ElkGraphJson::for_graph_shared(shared.clone())
            .remember_importer(&mut importer_slot)
            .lenient(false)
            .to_elk()
            .map_err(|err| format!("{err}"))?;

        ElkLayered::reset_trace_step_counter();

        let mut engine = RecursiveGraphLayoutEngine::new();
        let mut monitor = BasicProgressMonitor::new();
        engine.layout(&root, &mut monitor);

        let importer = importer_slot
            .get_mut()
            .ok_or_else(|| "internal error: missing json importer slot".to_string())?;
        importer
            .transfer_layout(&root)
            .map_err(|err| format!("{err}"))?;

        let serialized = serde_json::to_string(&*shared.borrow())
            .map_err(|err| format!("Failed to serialize JSON: {err}"))?;

        Ok(serialized)
    }));

    match result {
            Ok(inner) => inner,
        Err(payload) => {
            let msg = panic_payload_to_string(&*payload);
            // If the panic message already contains an ELK exception class name,
            // pass it through; otherwise wrap with UnsupportedConfigurationException
            // (matching Java ELK's RecursiveGraphLayoutEngine behavior).
            if msg.contains("org.eclipse.elk.core.") {
                Err(msg)
            } else {
                Err(format!(
                    "org.eclipse.elk.core.UnsupportedConfigurationException: {msg}"
                ))
            }
        }
    }
}

fn graph_feature_name(feature: &GraphFeature) -> &'static str {
    match feature {
        GraphFeature::SelfLoops => "SELF_LOOPS",
        GraphFeature::InsideSelfLoops => "INSIDE_SELF_LOOPS",
        GraphFeature::MultiEdges => "MULTI_EDGES",
        GraphFeature::EdgeLabels => "EDGE_LABELS",
        GraphFeature::Ports => "PORTS",
        GraphFeature::Compound => "COMPOUND",
        GraphFeature::Clusters => "CLUSTERS",
        GraphFeature::Disconnected => "DISCONNECTED",
    }
}

fn option_type_name(
    t: org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutOptionType,
) -> &'static str {
    use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutOptionType;
    match t {
        LayoutOptionType::Undefined => "UNDEFINED",
        LayoutOptionType::Boolean => "BOOLEAN",
        LayoutOptionType::Int => "INT",
        LayoutOptionType::String => "STRING",
        LayoutOptionType::Double => "DOUBLE",
        LayoutOptionType::Enum => "ENUM",
        LayoutOptionType::EnumSet => "ENUMSET",
        LayoutOptionType::Object => "OBJECT",
    }
}

fn option_target_name(
    t: &org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutOptionTarget,
) -> &'static str {
    use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutOptionTarget;
    match t {
        LayoutOptionTarget::Parents => "PARENTS",
        LayoutOptionTarget::Nodes => "NODES",
        LayoutOptionTarget::Edges => "EDGES",
        LayoutOptionTarget::Ports => "PORTS",
        LayoutOptionTarget::Labels => "LABELS",
    }
}

/// Return all registered layout algorithms as a JSON array.
pub fn known_layout_algorithms() -> Result<String, String> {
    ensure_initialized();

    let service = LayoutMetaDataService::get_instance();
    let algorithms = service.get_algorithm_data_list();

    let result: Vec<Value> = algorithms
        .iter()
        .map(|alg| {
            let known_options: Vec<Value> = alg
                .known_option_ids()
                .map(|id| Value::String(id.clone()))
                .collect();
            let supported_features: Vec<Value> = alg
                .supported_features()
                .iter()
                .map(|f| Value::String(graph_feature_name(f).to_string()))
                .collect();
            json!({
                "id": alg.id(),
                "name": alg.name(),
                "description": alg.description(),
                "category": alg.category_id().unwrap_or(""),
                "knownOptions": known_options,
                "supportedFeatures": supported_features,
            })
        })
        .collect();

    serde_json::to_string(&result).map_err(|e| format!("Failed to serialize algorithms: {e}"))
}

/// Return all registered layout options as a JSON array.
pub fn known_layout_options() -> Result<String, String> {
    ensure_initialized();

    let service = LayoutMetaDataService::get_instance();
    let options = service.get_option_data_list();

    let result: Vec<Value> = options
        .iter()
        .map(|opt| {
            let targets: Vec<Value> = opt
                .targets()
                .iter()
                .map(|t| Value::String(option_target_name(t).to_string()))
                .collect();
            json!({
                "id": opt.id(),
                "name": opt.name(),
                "description": opt.description(),
                "group": opt.group(),
                "type": option_type_name(opt.option_type()),
                "targets": targets,
            })
        })
        .collect();

    serde_json::to_string(&result).map_err(|e| format!("Failed to serialize options: {e}"))
}

/// Return all registered layout categories as a JSON array.
pub fn known_layout_categories() -> Result<String, String> {
    ensure_initialized();

    let service = LayoutMetaDataService::get_instance();
    let categories = service.get_category_data_list();

    let result: Vec<Value> = categories
        .iter()
        .map(|cat| {
            let known_layouters: Vec<Value> = cat
                .layouters()
                .iter()
                .map(|l| Value::String(l.id().to_string()))
                .collect();
            json!({
                "id": cat.id(),
                "name": cat.name(),
                "description": cat.description(),
                "knownLayouters": known_layouters,
            })
        })
        .collect();

    serde_json::to_string(&result).map_err(|e| format!("Failed to serialize categories: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_layout() {
        let graph = r#"{
            "id": "root",
            "layoutOptions": { "elk.direction": "RIGHT" },
            "children": [
                { "id": "n1", "width": 10, "height": 10 },
                { "id": "n2", "width": 10, "height": 10 }
            ],
            "edges": [{
                "id": "e1",
                "sources": ["n1"],
                "targets": ["n2"]
            }]
        }"#;

        let result = layout_json(graph, "{}").unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert!(value["children"][0]["x"].as_f64().is_some());
    }

    #[test]
    fn test_global_options_as_defaults() {
        let graph = r#"{
            "id": "root",
            "layoutOptions": { "elk.direction": "RIGHT" },
            "children": [
                { "id": "n1", "width": 10, "height": 10 },
                { "id": "n2", "width": 10, "height": 10 }
            ],
            "edges": [{
                "id": "e1",
                "sources": ["n1"],
                "targets": ["n2"]
            }]
        }"#;

        // Global option should NOT override concrete element option
        let result = layout_json(graph, r#"{"org.eclipse.elk.direction": "DOWN"}"#).unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        // Layout is RIGHT, so children have same y but different x
        let y0 = value["children"][0]["y"].as_f64().unwrap();
        let y1 = value["children"][1]["y"].as_f64().unwrap();
        assert_eq!(y0, y1, "RIGHT layout should produce same y for both nodes");
    }

    #[test]
    fn test_known_algorithms() {
        let result = known_layout_algorithms().unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert!(!value.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_known_options() {
        let result = known_layout_options().unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert!(!value.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_known_categories() {
        let result = known_layout_categories().unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert!(!value.as_array().unwrap().is_empty());
    }
}
