use serde_json::{Map, Number, Value};

use super::json_import_exception::{JsonImportError, JsonImportException};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum JsonId {
    String(String),
    Int(i64),
}

impl JsonId {
    pub fn as_string(&self) -> String {
        match self {
            JsonId::String(value) => value.clone(),
            JsonId::Int(value) => value.to_string(),
        }
    }
}

pub struct JsonAdapter;

impl JsonAdapter {
    pub fn get_id(obj: &Map<String, Value>) -> Result<JsonId, JsonImportError> {
        let value = obj.get("id").ok_or_else(|| {
            JsonImportError::from(JsonImportException::new("Every element must have an id."))
        })?;
        Self::as_id(value)
    }

    pub fn as_id(value: &Value) -> Result<JsonId, JsonImportError> {
        match value {
            Value::String(text) => Ok(JsonId::String(text.clone())),
            Value::Number(number) => {
                if let Some(id) = number.as_i64() {
                    return Ok(JsonId::Int(id));
                }
                if let Some(id) = number.as_f64() {
                    if is_int(id) {
                        return Ok(JsonId::Int(id as i64));
                    }
                }
                Err(JsonImportError::from(JsonImportException::new(
                    format!("Id must be a string or an integer: '{value}'."),
                )))
            }
            _ => Err(JsonImportError::from(JsonImportException::new(
                format!("Id must be a string or an integer: '{value}'."),
            ))),
        }
    }

    pub fn id_save(obj: &Map<String, Value>) -> Result<Option<String>, JsonImportError> {
        Self::opt_string(obj, "id")
    }

    pub fn string_val(value: &Value) -> Result<String, JsonImportError> {
        match value {
            Value::String(text) => Ok(text.clone()),
            Value::Number(number) => Ok(number_to_string(number)),
            Value::Bool(flag) => Ok(flag.to_string()),
            Value::Null => Ok("null".to_string()),
            _ => Err(JsonImportError::from(JsonImportException::new(
                "Expected a primitive JSON value.".to_string(),
            ))),
        }
    }

    pub fn opt_string(
        obj: &Map<String, Value>,
        key: &str,
    ) -> Result<Option<String>, JsonImportError> {
        match obj.get(key) {
            Some(value) => Ok(Some(Self::string_val(value)?)),
            None => Ok(None),
        }
    }

    pub fn opt_double(
        obj: &Map<String, Value>,
        key: &str,
    ) -> Result<Option<f64>, JsonImportError> {
        match obj.get(key) {
            Some(value) => match value {
                Value::Number(number) => number
                    .as_f64()
                    .ok_or_else(|| {
                        JsonImportError::from(JsonImportException::new(
                            "Invalid number value.".to_string(),
                        ))
                    })
                    .map(Some),
                Value::String(text) => text
                    .parse::<f64>()
                    .map(Some)
                    .map_err(|_| JsonImportError::from(JsonImportException::new("Invalid number value."))),
                Value::Null => Ok(None),
                _ => Err(JsonImportError::from(JsonImportException::new(
                    "Invalid number value.".to_string(),
                ))),
            },
            None => Ok(None),
        }
    }

    pub fn opt_json_array<'a>(
        obj: &'a Map<String, Value>,
        key: &str,
    ) -> Option<&'a Vec<Value>> {
        obj.get(key)?.as_array()
    }

    pub fn opt_json_object<'a>(
        obj: &'a Map<String, Value>,
        key: &str,
    ) -> Option<&'a Map<String, Value>> {
        obj.get(key)?.as_object()
    }

    pub fn opt_json_object_in_array(
        arr: &[Value],
        index: usize,
    ) -> Option<&Map<String, Value>> {
        arr.get(index)?.as_object()
    }

    pub fn has_key(obj: &Map<String, Value>, key: &str) -> bool {
        obj.contains_key(key)
    }
}

fn is_int(value: f64) -> bool {
    value.is_finite() && value.fract() == 0.0
}

fn number_to_string(number: &Number) -> String {
    if let Some(value) = number.as_i64() {
        return value.to_string();
    }
    if let Some(value) = number.as_u64() {
        return value.to_string();
    }
    if let Some(value) = number.as_f64() {
        return value.to_string();
    }
    number.to_string()
}
