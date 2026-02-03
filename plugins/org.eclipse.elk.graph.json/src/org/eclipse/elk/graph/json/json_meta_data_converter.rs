use serde_json::{Map, Value};

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaData, LayoutAlgorithmData, LayoutCategoryData, LayoutOptionData,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::GraphFeature;

pub struct JsonMetaDataConverter;

impl JsonMetaDataConverter {
    pub fn to_json_algorithm(data: &LayoutAlgorithmData) -> Value {
        let mut obj = create_common(data);
        if let Some(category) = data.category_id() {
            if !category.is_empty() {
                obj.insert("category".to_string(), Value::String(category.to_string()));
            }
        }

        let known_options: Vec<_> = data.known_option_ids().cloned().collect();
        if !known_options.is_empty() {
            let arr = known_options
                .into_iter()
                .map(Value::String)
                .collect::<Vec<_>>();
            obj.insert("knownOptions".to_string(), Value::Array(arr));
        }

        if !data.supported_features().is_empty() {
            let arr = data
                .supported_features()
                .iter()
                .map(|feature| Value::String(graph_feature_to_string(feature)))
                .collect::<Vec<_>>();
            obj.insert("supportedFeatures".to_string(), Value::Array(arr));
        }

        Value::Object(obj)
    }

    pub fn to_json_category(data: &LayoutCategoryData) -> Value {
        let mut obj = create_common(data);
        if !data.layouters().is_empty() {
            let arr = data
                .layouters()
                .iter()
                .filter_map(|layouter| {
                    let id = layouter.id();
                    if id.is_empty() {
                        None
                    } else {
                        Some(Value::String(id.to_string()))
                    }
                })
                .collect::<Vec<_>>();
            if !arr.is_empty() {
                obj.insert("knownLayouters".to_string(), Value::Array(arr));
            }
        }
        Value::Object(obj)
    }

    pub fn to_json_option(data: &LayoutOptionData) -> Value {
        let mut obj = create_common(data);
        if !data.group().is_empty() {
            obj.insert("group".to_string(), Value::String(data.group().to_string()));
        }
        let option_type = enum_to_string(&data.option_type());
        obj.insert("type".to_string(), Value::String(option_type));

        if !data.targets().is_empty() {
            let arr = data
                .targets()
                .iter()
                .map(|target| Value::String(enum_to_string(target)))
                .collect::<Vec<_>>();
            obj.insert("targets".to_string(), Value::Array(arr));
        }

        Value::Object(obj)
    }
}

fn create_common(data: &dyn ILayoutMetaData) -> Map<String, Value> {
    let mut obj = Map::new();
    if !data.id().is_empty() {
        obj.insert("id".to_string(), Value::String(data.id().to_string()));
    }
    if !data.name().is_empty() {
        obj.insert("name".to_string(), Value::String(data.name().to_string()));
    }
    if !data.description().is_empty() {
        obj.insert(
            "description".to_string(),
            Value::String(data.description().to_string()),
        );
    }
    obj
}

fn enum_to_string<T: std::fmt::Debug>(value: &T) -> String {
    to_upper_snake(&format!("{:?}", value))
}

fn to_upper_snake(value: &str) -> String {
    let mut out = String::new();
    let mut prev: Option<char> = None;
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        let next = chars.peek().copied();
        if let Some(prev_ch) = prev {
            if ch.is_uppercase()
                && (prev_ch.is_lowercase() || next.map(|n| n.is_lowercase()).unwrap_or(false))
            {
                out.push('_');
            }
        }
        out.push(ch.to_ascii_uppercase());
        prev = Some(ch);
    }
    out
}

fn graph_feature_to_string(feature: &GraphFeature) -> String {
    match feature {
        GraphFeature::SelfLoops => "SELF_LOOPS".to_string(),
        GraphFeature::InsideSelfLoops => "INSIDE_SELF_LOOPS".to_string(),
        GraphFeature::MultiEdges => "MULTI_EDGES".to_string(),
        GraphFeature::EdgeLabels => "EDGE_LABELS".to_string(),
        GraphFeature::Ports => "PORTS".to_string(),
        GraphFeature::Compound => "COMPOUND".to_string(),
        GraphFeature::Clusters => "CLUSTERS".to_string(),
        GraphFeature::Disconnected => "DISCONNECTED".to_string(),
    }
}
