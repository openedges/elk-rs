use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::org::eclipse::elk::core::data::ILayoutMetaData;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayoutOptionType {
    Undefined,
    Boolean,
    Int,
    String,
    Double,
    Enum,
    EnumSet,
    Object,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayoutOptionTarget {
    Parents,
    Nodes,
    Edges,
    Ports,
    Labels,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayoutOptionVisibility {
    Visible,
    Advanced,
    Hidden,
}

type ParseFn = Arc<dyn Fn(&str) -> Option<Arc<dyn Any + Send + Sync>> + Send + Sync>;

#[derive(Clone)]
pub struct LayoutOptionDependency {
    option: LayoutOptionData,
    required_value: Option<Arc<dyn Any + Send + Sync>>,
}

impl LayoutOptionDependency {
    pub fn new(
        option: LayoutOptionData,
        required_value: Option<Arc<dyn Any + Send + Sync>>,
    ) -> Self {
        LayoutOptionDependency {
            option,
            required_value,
        }
    }

    pub fn option(&self) -> &LayoutOptionData {
        &self.option
    }

    pub fn required_value(&self) -> Option<Arc<dyn Any + Send + Sync>> {
        self.required_value.as_ref().map(Arc::clone)
    }
}

#[derive(Clone)]
pub struct LayoutOptionData {
    id: String,
    group: String,
    legacy_ids: Vec<String>,
    default_value: Option<Arc<dyn Any + Send + Sync>>,
    option_type: LayoutOptionType,
    name: String,
    description: String,
    targets: HashSet<LayoutOptionTarget>,
    dependencies: Vec<LayoutOptionDependency>,
    visibility: LayoutOptionVisibility,
    lower_bound: Option<Arc<dyn Any + Send + Sync>>,
    upper_bound: Option<Arc<dyn Any + Send + Sync>>,
    choices: Option<Vec<String>>,
    value_type_id: Option<TypeId>,
    parser: Option<ParseFn>,
}

impl LayoutOptionData {
    pub fn builder() -> LayoutOptionDataBuilder {
        LayoutOptionDataBuilder::new()
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn group(&self) -> &str {
        &self.group
    }

    pub fn legacy_ids(&self) -> &[String] {
        &self.legacy_ids
    }

    pub fn option_type(&self) -> LayoutOptionType {
        self.option_type
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn targets(&self) -> &HashSet<LayoutOptionTarget> {
        &self.targets
    }

    pub fn visibility(&self) -> LayoutOptionVisibility {
        self.visibility
    }

    pub fn value_type_id(&self) -> Option<TypeId> {
        self.value_type_id
    }

    pub fn default_value(&self) -> Option<Arc<dyn Any + Send + Sync>> {
        self.default_value.as_ref().map(Arc::clone)
    }

    pub fn lower_bound(&self) -> Option<Arc<dyn Any + Send + Sync>> {
        self.lower_bound.as_ref().map(Arc::clone)
    }

    pub fn upper_bound(&self) -> Option<Arc<dyn Any + Send + Sync>> {
        self.upper_bound.as_ref().map(Arc::clone)
    }

    pub fn set_lower_bound(&mut self, lower_bound: Option<Arc<dyn Any + Send + Sync>>) {
        self.lower_bound = lower_bound;
    }

    pub fn set_upper_bound(&mut self, upper_bound: Option<Arc<dyn Any + Send + Sync>>) {
        self.upper_bound = upper_bound;
    }

    pub fn dependencies(&self) -> &[LayoutOptionDependency] {
        &self.dependencies
    }

    pub fn dependencies_mut(&mut self) -> &mut Vec<LayoutOptionDependency> {
        &mut self.dependencies
    }

    pub fn can_parse_value(&self) -> bool {
        match self.option_type {
            LayoutOptionType::Undefined => false,
            LayoutOptionType::Enum | LayoutOptionType::EnumSet | LayoutOptionType::Object => {
                self.parser.is_some()
            }
            _ => true,
        }
    }

    pub fn parse_value(&self, value_string: &str) -> Option<Arc<dyn Any + Send + Sync>> {
        if value_string == "null" {
            return None;
        }
        if value_string.is_empty() && self.option_type != LayoutOptionType::EnumSet {
            return None;
        }

        match self.option_type {
            LayoutOptionType::Boolean => {
                if value_string.eq_ignore_ascii_case("true") {
                    Some(Arc::new(true))
                } else if value_string.eq_ignore_ascii_case("false") {
                    Some(Arc::new(false))
                } else {
                    None
                }
            }
            LayoutOptionType::Int => value_string
                .parse::<i32>()
                .ok()
                .map(|value| Arc::new(value) as Arc<dyn Any + Send + Sync>),
            LayoutOptionType::Double => value_string
                .parse::<f64>()
                .ok()
                .map(|value| Arc::new(value) as Arc<dyn Any + Send + Sync>),
            LayoutOptionType::String => {
                Some(Arc::new(value_string.to_string()) as Arc<dyn Any + Send + Sync>)
            }
            LayoutOptionType::Enum | LayoutOptionType::EnumSet | LayoutOptionType::Object => {
                let parser = self.parser.as_ref()?;
                parser(value_string)
            }
            LayoutOptionType::Undefined => None,
        }
    }

    pub fn default_default_value(&self) -> Option<Arc<dyn Any + Send + Sync>> {
        match self.option_type {
            LayoutOptionType::String => Some(Arc::new(String::new())),
            LayoutOptionType::Boolean => Some(Arc::new(false)),
            LayoutOptionType::Int => Some(Arc::new(0_i32)),
            LayoutOptionType::Double => Some(Arc::new(0.0_f64)),
            LayoutOptionType::Enum | LayoutOptionType::EnumSet | LayoutOptionType::Object => None,
            LayoutOptionType::Undefined => None,
        }
    }

    pub fn choices(&self) -> Vec<String> {
        match self.option_type {
            LayoutOptionType::Boolean => vec!["false".to_string(), "true".to_string()],
            LayoutOptionType::Enum | LayoutOptionType::EnumSet => self.choices.clone().unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    pub fn enum_value_count(&self) -> usize {
        match self.option_type {
            LayoutOptionType::Enum | LayoutOptionType::EnumSet => {
                self.choices.as_ref().map(|v| v.len()).unwrap_or(0)
            }
            _ => 0,
        }
    }

    pub fn enum_value_name(&self, index: usize) -> Option<&str> {
        self.choices
            .as_ref()
            .and_then(|values| values.get(index))
            .map(|value| value.as_str())
    }
}

impl ILayoutMetaData for LayoutOptionData {
    fn id(&self) -> &str {
        self.id()
    }

    fn name(&self) -> &str {
        self.name()
    }

    fn description(&self) -> &str {
        self.description()
    }
}

impl std::fmt::Debug for LayoutOptionData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutOptionData").field("id", &self.id).finish()
    }
}

impl PartialEq for LayoutOptionData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for LayoutOptionData {}

impl Hash for LayoutOptionData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct LayoutOptionDataBuilder {
    id: Option<String>,
    group: Option<String>,
    legacy_ids: Vec<String>,
    default_value: Option<Arc<dyn Any + Send + Sync>>,
    option_type: LayoutOptionType,
    name: Option<String>,
    description: Option<String>,
    targets: Option<HashSet<LayoutOptionTarget>>,
    visibility: LayoutOptionVisibility,
    lower_bound: Option<Arc<dyn Any + Send + Sync>>,
    upper_bound: Option<Arc<dyn Any + Send + Sync>>,
    choices: Option<Vec<String>>,
    value_type_id: Option<TypeId>,
    parser: Option<ParseFn>,
}

impl LayoutOptionDataBuilder {
    pub fn new() -> Self {
        LayoutOptionDataBuilder {
            id: None,
            group: None,
            legacy_ids: Vec::new(),
            default_value: None,
            option_type: LayoutOptionType::Undefined,
            name: None,
            description: None,
            targets: None,
            visibility: LayoutOptionVisibility::Visible,
            lower_bound: None,
            upper_bound: None,
            choices: None,
            value_type_id: None,
            parser: None,
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn legacy_ids(mut self, legacy_ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.legacy_ids = legacy_ids.into_iter().map(Into::into).collect();
        self
    }

    pub fn default_value(mut self, default_value: Option<Arc<dyn Any + Send + Sync>>) -> Self {
        self.default_value = default_value;
        self
    }

    pub fn option_type(mut self, option_type: LayoutOptionType) -> Self {
        self.option_type = option_type;
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn targets(mut self, targets: HashSet<LayoutOptionTarget>) -> Self {
        self.targets = Some(targets);
        self
    }

    pub fn visibility(mut self, visibility: LayoutOptionVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn lower_bound(mut self, lower_bound: Option<Arc<dyn Any + Send + Sync>>) -> Self {
        self.lower_bound = lower_bound;
        self
    }

    pub fn upper_bound(mut self, upper_bound: Option<Arc<dyn Any + Send + Sync>>) -> Self {
        self.upper_bound = upper_bound;
        self
    }

    pub fn choices(mut self, choices: Vec<String>) -> Self {
        self.choices = Some(choices);
        self
    }

    pub fn value_type_id(mut self, value_type_id: TypeId) -> Self {
        self.value_type_id = Some(value_type_id);
        self
    }

    pub fn parser(
        mut self,
        parser: ParseFn,
    ) -> Self {
        self.parser = Some(parser);
        self
    }

    pub fn create(self) -> LayoutOptionData {
        LayoutOptionData {
            id: self.id.unwrap_or_default(),
            group: self.group.unwrap_or_default(),
            legacy_ids: self.legacy_ids,
            default_value: self.default_value,
            option_type: self.option_type,
            name: self.name.unwrap_or_default(),
            description: self.description.unwrap_or_default(),
            targets: self.targets.unwrap_or_default(),
            dependencies: Vec::new(),
            visibility: self.visibility,
            lower_bound: self.lower_bound,
            upper_bound: self.upper_bound,
            choices: self.choices,
            value_type_id: self.value_type_id,
            parser: self.parser,
        }
    }
}

impl Default for LayoutOptionDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}
