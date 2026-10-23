//! Public layout API for WASM and NAPI bindings.
//!
//! Provides the core layout pipeline functions shared by both wasm-bindgen
//! and napi-rs wrappers.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
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
    AlgorithmFactory, BasicProgressMonitor, InstancePool,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::GraphFeature;

use crate::org::eclipse::elk::graph::json::JsonImporter;

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

/// One immutable set of JSON defaults per invocation. Canonical lookup results
/// are cached separately; no option values are cloned until result transfer.
#[derive(Default)]
pub(crate) struct GlobalOptions {
    values: Map<String, Value>,
    canonical_ids: RefCell<HashMap<String, String>>,
}

impl GlobalOptions {
    pub(crate) fn parse(options_json: &str) -> Result<Self, String> {
        if options_json.is_empty() {
            return Ok(Self::default());
        }
        let global: Value = serde_json::from_str(options_json)
            .map_err(|e| format!("Failed to parse options JSON: {e}"))?;
        let mut canonical_ids = HashMap::new();
        let mut seen = std::collections::HashSet::new();
        let mut values = Map::new();
        if let Value::Object(options) = global {
            for (key, value) in options {
                let canonical = canonical_option_id(&mut canonical_ids, &key);
                if seen.insert(canonical) {
                    values.insert(key, value);
                }
            }
        }
        Ok(Self {
            values,
            canonical_ids: RefCell::new(canonical_ids),
        })
    }

    pub(crate) fn visit_effective<E>(
        &self,
        element: &Map<String, Value>,
        mut visit: impl FnMut(&str, &Value) -> Result<(), E>,
    ) -> Result<(), E> {
        let selected = element
            .get("layoutOptions")
            .or_else(|| element.get("properties"));
        // A present non-object field blocks merging, even if the importer can
        // fall back from malformed layoutOptions to an authored properties map.
        if self.values.is_empty() || selected.is_some_and(|value| !value.is_object()) {
            if let Some(options) = element
                .get("layoutOptions")
                .and_then(Value::as_object)
                .or_else(|| element.get("properties").and_then(Value::as_object))
            {
                for (key, value) in options {
                    visit(key, value)?;
                }
            }
            return Ok(());
        }
        let authored = selected.and_then(Value::as_object);
        let mut ids = self.canonical_ids.borrow_mut();
        let mut existing = std::collections::HashSet::new();
        // Retain only temporary keys, not cloned JSON values. Using serde's
        // own Map preserves both sorted and preserve_order iteration semantics.
        let mut keys = Map::new();
        if let Some(authored) = authored {
            for key in authored.keys() {
                existing.insert(canonical_option_id(&mut ids, key));
                keys.insert(key.clone(), Value::Null);
            }
        }
        for key in self.values.keys() {
            if !existing.contains(&canonical_option_id(&mut ids, key)) {
                keys.insert(key.clone(), Value::Null);
            }
        }
        drop(ids);
        for key in keys.keys() {
            let value = authored
                .and_then(|options| options.get(key))
                .unwrap_or_else(|| &self.values[key]);
            visit(key, value)?;
        }
        Ok(())
    }

    pub(crate) fn materialize(&self, root: &mut Value) {
        if self.values.is_empty() {
            return;
        }
        let mut ids = self.canonical_ids.borrow_mut();
        let resolved: Vec<_> = self
            .values
            .iter()
            .map(|(key, value)| (key.as_str(), value, canonical_option_id(&mut ids, key)))
            .collect();
        apply_global_options_recursive(root, &resolved, &mut ids);
    }
}

/// Merge global layout options into every element of the graph JSON as defaults.
///
/// Global options are applied recursively to the root and all descendant
/// elements (children, edges, ports, labels). For each element, a global
/// option is only added if no existing option with the same canonical ID
/// is already present. This correctly handles option name aliases
/// (e.g. `elk.direction` vs `org.eclipse.elk.direction`) by resolving
/// all keys to canonical IDs via the LayoutMetaDataService.
#[cfg(test)]
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

    // Resolve each distinct spelling once per merge, rather than cloning the
    // full metadata (including dependencies) for every port, edge and node.
    let mut canonical_ids = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    let resolved: Vec<_> = global_opts
        .iter()
        .filter_map(|(key, value)| {
            let canonical = canonical_option_id(&mut canonical_ids, key);
            seen.insert(canonical.clone())
                .then_some((key.as_str(), value, canonical))
        })
        .collect();
    apply_global_options_recursive(root_value, &resolved, &mut canonical_ids);
    Ok(())
}

fn canonical_option_id(ids: &mut HashMap<String, String>, key: &str) -> String {
    if let Some(canonical) = ids.get(key) {
        return canonical.clone();
    }
    let canonical = LayoutMetaDataService::get_instance()
        .get_option_data_by_suffix(key)
        .map(|option| option.id().to_owned())
        .unwrap_or_else(|| key.to_owned());
    ids.insert(key.to_owned(), canonical.clone());
    canonical
}

/// Recursively apply global layout options to an element and all its
/// descendants (children, edges, ports, labels).
fn apply_global_options_recursive(
    element: &mut Value,
    global_opts: &[(&str, &Value, String)],
    canonical_ids: &mut HashMap<String, String>,
) {
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

        let mut existing_canonical_ids = std::collections::HashSet::new();
        if let Some(Value::Object(existing)) = obj.get(field) {
            for key in existing.keys() {
                existing_canonical_ids.insert(canonical_option_id(canonical_ids, key));
            }
        }

        let layout_options = obj
            .entry(field)
            .or_insert_with(|| Value::Object(Map::new()));
        if let Some(lo) = layout_options.as_object_mut() {
            for (key, value, canonical) in global_opts {
                if !existing_canonical_ids.contains(canonical) {
                    lo.insert((*key).to_owned(), (*value).clone());
                    // Global aliases were deduplicated before this walk;
                    // only authored options need membership storage here.
                }
            }
        }
    }

    // Step 2: recurse into sub-elements (children, edges, ports, labels)
    for arr_key in &["children", "edges", "ports", "labels"] {
        if let Some(arr) = element.get_mut(*arr_key) {
            if let Some(items) = arr.as_array_mut() {
                for item in items {
                    apply_global_options_recursive(item, global_opts, canonical_ids);
                }
            }
        }
    }
}

#[cfg(test)]
fn reference_merge_global_options(
    root_value: &mut Value,
    options_json: &str,
) -> Result<(), String> {
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

    reference_apply_global_options_recursive(root_value, global_opts);
    Ok(())
}

/// Recursively apply global layout options to an element and all its
/// descendants (children, edges, ports, labels).
#[cfg(test)]
fn reference_apply_global_options_recursive(element: &mut Value, global_opts: &Map<String, Value>) {
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
                    reference_apply_global_options_recursive(item, global_opts);
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
    catch_layout(|| {
        let input_value: Value = serde_json::from_str(graph_json)
            .map_err(|err| format!("Failed to parse graph JSON: {err}"))?;
        let output = layout_value(input_value, options_json)?;
        serde_json::to_string(&output).map_err(|err| format!("Failed to serialize JSON: {err}"))
    })
}

/// The same pipeline for callers that already own JSON values. Layout options,
/// importer traversal, engine phases and panic translation match layout_json.
pub fn layout_value(input_value: Value, options_json: &str) -> Result<Value, String> {
    ensure_initialized();

    catch_layout(|| {
        let defaults = Rc::new(GlobalOptions::parse(options_json)?);
        let shared = Rc::new(RefCell::new(input_value));
        let mut importer = JsonImporter::new();
        let root = importer
            .transform_shared_with_defaults(shared.clone(), defaults.clone())
            .map_err(|err| format!("{err}"))?;
        importer.release_import_scratch();

        ElkLayered::reset_trace_step_counter();

        let mut engine = RecursiveGraphLayoutEngine::new();
        let mut monitor = BasicProgressMonitor::new();
        engine.layout(&root, &mut monitor);

        // Transfer reads some raw option spellings as well as preserving the
        // complete input JSON. Restore exactly the old merged representation,
        // after the layered working graphs have been released by the provider.
        defaults.materialize(&mut shared.borrow_mut());
        importer
            .transfer_layout(&root)
            .map_err(|err| format!("{err}"))?;

        // Transfer is complete. Importer/graph teardown does not read this
        // JSON again; take the owned value without cloning its property maps.
        let output = std::mem::take(&mut *shared.borrow_mut());
        Ok(output)
    })
}

fn catch_layout<T>(operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let result = panic::catch_unwind(AssertUnwindSafe(operation));
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
    fn shared_defaults_and_owned_values_preserve_every_registered_provider() {
        for algorithm in [
            "layered",
            "force",
            "stress",
            "mrtree",
            "radial",
            "rectpacking",
            "disco",
            "sporeOverlap",
            "sporeCompaction",
            "vertiflex",
        ] {
            let graph = json!({
                "id":"root", "custom":{"keep":true},
                "children":[{"id":"a","width":30,"height":20},{"id":"b","width":40,"height":25}],
                "edges":[{"id":"ab","sources":["a"],"targets":["b"]}]
            });
            let options =
                json!({"elk.algorithm":algorithm,"elk.randomSeed":1,"elk.spacing.nodeNode":35})
                    .to_string();
            // DisCo's default polyomino option has an existing reflection error;
            // preserve its error as well as every successful provider's output.
            let expected = eager_layout(graph.clone(), &options)
                .map(|value| serde_json::to_string(&value).unwrap());
            if let Err(error) = &expected {
                assert_eq!(algorithm, "disco");
                assert!(error.contains("org.eclipse.elk.polyomino.lowLevelSort"));
            }
            let actual = layout_value(graph.clone(), &options)
                .map(|value| serde_json::to_string(&value).unwrap());
            assert_eq!(actual, expected, "{algorithm}");
            assert_eq!(
                layout_json(&graph.to_string(), &options),
                expected,
                "{algorithm}"
            );
        }
    }

    #[test]
    fn owned_value_errors_match_the_eager_pipeline() {
        for graph in [
            Value::Null,
            json!([]),
            json!({"id":"root","edges":[{"id":"e","sources":["missing"],"targets":["also-missing"]}]}),
        ] {
            let expected = eager_layout(graph.clone(), "{}").unwrap_err();
            assert_eq!(layout_value(graph, "{}").unwrap_err(), expected);
        }
    }

    // Reference eager-import path: actually materialize defaults before plain import,
    // and retain the original import indexes through layout and transfer.
    fn eager_layout(mut input: Value, options: &str) -> Result<Value, String> {
        ensure_initialized();
        catch_layout(|| {
            reference_merge_global_options(&mut input, options)?;
            let shared = Rc::new(RefCell::new(input));
            let mut importer = JsonImporter::new();
            let root = importer
                .transform_shared(shared.clone())
                .map_err(|e| e.to_string())?;
            ElkLayered::reset_trace_step_counter();
            RecursiveGraphLayoutEngine::new().layout(&root, &mut BasicProgressMonitor::new());
            importer.transfer_layout(&root).map_err(|e| e.to_string())?;
            let result = std::mem::take(&mut *shared.borrow_mut());
            Ok(result)
        })
    }

    #[test]
    fn shared_defaults_match_eager_options_and_output() {
        ensure_initialized();
        let options = r#"{"elk.algorithm":"layered","elk.direction":"DOWN","org.eclipse.elk.direction":"UP","unknown.option":"global","elk.port.side":"EAST","elk.spacing.nodeNode":35}"#;
        // Compare raw effective option order as well as values, for both serde
        // Map backends (workspace preserve_order and standalone sorted maps).
        for element in [
            json!({}),
            json!({"layoutOptions":{"org.eclipse.elk.direction":"RIGHT","unknown.option":"local"}}),
            json!({"properties":{"direction":"LEFT","elk.direction":"RIGHT"}}),
            json!({"layoutOptions":null,"properties":{"elk.direction":"LEFT"}}),
            json!({"properties":false}),
            json!({"layoutOptions":{},"properties":{"elk.direction":"LEFT"}}),
        ] {
            let mut eager = element.clone();
            reference_merge_global_options(&mut eager, options).unwrap();
            let expected = eager
                .get("layoutOptions")
                .and_then(Value::as_object)
                .or_else(|| eager.get("properties").and_then(Value::as_object));
            let expected: Vec<_> = expected
                .into_iter()
                .flat_map(|map| map.iter())
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            let defaults = GlobalOptions::parse(options).unwrap();
            let mut actual = Vec::new();
            defaults
                .visit_effective(element.as_object().unwrap(), |key, value| {
                    actual.push((key.to_owned(), value.clone()));
                    Ok::<_, ()>(())
                })
                .unwrap();
            assert_eq!(actual, expected);
        }

        let graph = json!({
            "id":"root", "custom":{"preserved":[1,2,3]},
            "layoutOptions":{"org.eclipse.elk.direction":"RIGHT","unknown.option":"authored"},
            "children":[
                {"id":"a","width":30,"height":20,"properties":{"direction":"LEFT"},
                 "ports":[{"id":"out","width":0,"height":0}],
                 "labels":[{"text":"A","width":8,"height":8}]},
                {"id":"b","width":40,"height":25,"ports":[{"id":"in","width":0,"height":0,"layoutOptions":{"elk.port.side":"WEST"}}]}
            ],
            "edges":[{"id":"ab","sources":["out"],"targets":["in"],
                      "labels":[{"text":"AB","width":16,"height":8}]}]
        });
        for globals in [
            "",
            "{}",
            "[]",
            options,
            r#"{"unknown.option":{"preserved":true}}"#,
            r#"{"elk.direction":"invalid","elk.spacing.nodeNode":null}"#,
        ] {
            assert_eq!(
                layout_value(graph.clone(), globals)
                    .map(|value| serde_json::to_string(&value).unwrap()),
                eager_layout(graph.clone(), globals)
                    .map(|value| serde_json::to_string(&value).unwrap()),
                "{globals}"
            );
        }
        let output = layout_value(graph.clone(), options).unwrap();
        assert_eq!(output["layoutOptions"]["unknown.option"], "authored");
        assert_eq!(output["children"][0]["properties"]["direction"], "LEFT");
        assert_eq!(
            output["children"][0]["ports"][0]["layoutOptions"]["unknown.option"],
            "global"
        );
        assert_eq!(
            output["children"][0]["labels"][0]["layoutOptions"]["unknown.option"],
            "global"
        );
        assert_eq!(
            output["edges"][0]["layoutOptions"]["unknown.option"],
            "global"
        );
        assert_eq!(
            output["edges"][0]["labels"][0]["layoutOptions"]["unknown.option"],
            "global"
        );
        assert_eq!(output["custom"], graph["custom"]);
        assert_eq!(
            serde_json::to_string(&output).unwrap(),
            serde_json::to_string(&eager_layout(graph, options).unwrap()).unwrap()
        );
    }

    #[test]
    fn owned_value_and_string_boundaries_preserve_layout_bytes_and_errors() {
        let graph = json!({
            "id":"root",
            "children":[
                {"id":"a","width":30.1,"height":20.3,"ports":[{"id":"out","width":0,"height":0,"layoutOptions":{"elk.port.side":"EAST"}}]},
                {"id":"b","width":40.5,"height":25.25,"ports":[{"id":"in","width":0,"height":0,"layoutOptions":{"elk.port.side":"WEST"}}]}
            ],
            "edges":[{"id":"ab","sources":["out"],"targets":["in"]}]
        });
        for options in [
            "{}",
            r#"{"elk.direction":"DOWN","org.eclipse.elk.direction":"RIGHT"}"#,
        ] {
            let legacy = layout_json(&graph.to_string(), options).unwrap();
            let owned = layout_value(graph.clone(), options).unwrap();
            assert_eq!(serde_json::to_string(&owned).unwrap(), legacy);
        }
        assert_eq!(
            layout_value(json!({}), "invalid").unwrap_err(),
            layout_json("{}", "invalid").unwrap_err()
        );
    }

    #[test]
    fn global_aliases_keep_first_spelling_and_authored_values_on_every_element() {
        ensure_initialized();
        let globals = r#"{"elk.direction":"DOWN","org.eclipse.elk.direction":"UP"}"#;
        let parsed: Value = serde_json::from_str(globals).unwrap();
        let (first, value) = parsed.as_object().unwrap().iter().next().unwrap();
        let mut graph = json!({
            "children": [{"properties":{"elk.direction":"LEFT"}, "ports":[{},{}]}],
            "edges":[{}], "labels":[{}]
        });
        merge_global_options(&mut graph, globals).unwrap();
        assert_eq!(
            graph["children"][0]["properties"],
            json!({"elk.direction":"LEFT"})
        );
        for element in [
            &graph,
            &graph["children"][0]["ports"][0],
            &graph["children"][0]["ports"][1],
            &graph["edges"][0],
            &graph["labels"][0],
        ] {
            assert_eq!(element["layoutOptions"].as_object().unwrap().len(), 1);
            assert_eq!(&element["layoutOptions"][first], value);
        }
        let once = graph.clone();
        merge_global_options(&mut graph, globals).unwrap();
        assert_eq!(graph, once);
    }

    #[test]
    fn repeated_global_options_resolve_once_and_preserve_recursive_alias_precedence() {
        ensure_initialized();
        let mut graph = json!({
            "layoutOptions": {"elk.direction": "RIGHT"},
            "children": (0..64).map(|_| json!({
                "properties": {"elk.direction": "LEFT", "unknown.option": "local"},
                "ports": [{}], "edges": [{}], "labels": [{}]
            })).collect::<Vec<_>>()
        });
        let globals = json!({"org.eclipse.elk.direction": "DOWN", "unknown.option": "global"});
        let mut ids = HashMap::new();
        let resolved: Vec<_> = globals
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.as_str(), value, canonical_option_id(&mut ids, key)))
            .collect();
        apply_global_options_recursive(&mut graph, &resolved, &mut ids);
        assert_eq!(ids.len(), 3); // Distinct spellings, independent of element count.
        assert_eq!(graph["layoutOptions"]["elk.direction"], "RIGHT");
        assert!(graph["layoutOptions"]
            .get("org.eclipse.elk.direction")
            .is_none());
        for child in graph["children"].as_array().unwrap() {
            assert_eq!(child["properties"]["elk.direction"], "LEFT");
            assert_eq!(child["properties"]["unknown.option"], "local");
            assert!(child["properties"]
                .get("org.eclipse.elk.direction")
                .is_none());
            for field in ["ports", "edges", "labels"] {
                assert_eq!(child[field][0]["layoutOptions"], globals);
            }
        }
        let previous = graph.clone();
        merge_global_options(&mut graph, &globals.to_string()).unwrap();
        assert_eq!(graph, previous);
    }

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
