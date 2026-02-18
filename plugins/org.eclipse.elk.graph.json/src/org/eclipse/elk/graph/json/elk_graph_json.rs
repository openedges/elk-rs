use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;

use org_eclipse_elk_core::org::eclipse::elk::core::util::Maybe;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use super::json_exporter::JsonExporter;
use super::json_import_exception::{JsonIOException, JsonImportError, JsonImportException};
use super::json_importer::JsonImporter;

pub struct ElkGraphJson;

impl ElkGraphJson {
    pub fn for_graph<'a>(graph: impl Into<String>) -> ImportBuilder<'a> {
        ImportBuilder::from_string(graph.into())
    }

    pub fn for_graph_value<'a>(graph: Value) -> ImportBuilder<'a> {
        ImportBuilder::from_value(graph)
    }

    pub fn for_graph_shared<'a>(graph: Rc<RefCell<Value>>) -> ImportBuilder<'a> {
        ImportBuilder::from_shared(graph)
    }

    pub fn for_elk(graph: ElkNodeRef) -> ExportBuilder {
        ExportBuilder::new(graph)
    }
}

pub struct ImportBuilder<'a> {
    graph: Option<GraphInput>,
    importer_slot: Option<&'a mut Maybe<JsonImporter>>,
    lenient: bool,
}

impl<'a> ImportBuilder<'a> {
    fn from_string(graph: String) -> Self {
        ImportBuilder {
            graph: Some(GraphInput::Text(graph)),
            importer_slot: None,
            lenient: true,
        }
    }

    fn from_value(graph: Value) -> Self {
        ImportBuilder {
            graph: Some(GraphInput::Value(Rc::new(RefCell::new(graph)))),
            importer_slot: None,
            lenient: true,
        }
    }

    fn from_shared(graph: Rc<RefCell<Value>>) -> Self {
        ImportBuilder {
            graph: Some(GraphInput::Value(graph)),
            importer_slot: None,
            lenient: true,
        }
    }

    pub fn remember_importer(mut self, slot: &'a mut Maybe<JsonImporter>) -> Self {
        self.importer_slot = Some(slot);
        self
    }

    pub fn lenient(mut self, be_lenient: bool) -> Self {
        self.lenient = be_lenient;
        self
    }

    pub fn to_elk(mut self) -> Result<ElkNodeRef, JsonImportError> {
        let input = self.graph.take().ok_or_else(|| {
            JsonImportError::from(JsonImportException::new("No input graph provided."))
        })?;

        let shared = match input {
            GraphInput::Value(value) => value,
            GraphInput::Text(text) => {
                let parsed = if self.lenient {
                    parse_lenient(&text)?
                } else {
                    serde_json::from_str(&text).map_err(|err| {
                        JsonImportError::from(JsonIOException::new(err.to_string()))
                    })?
                };
                Rc::new(RefCell::new(parsed))
            }
        };

        let mut importer = JsonImporter::new();
        let root = importer.transform_shared(shared.clone())?;

        if let Some(slot) = self.importer_slot.as_mut() {
            slot.set(importer);
        }

        Ok(root)
    }
}

pub struct ExportBuilder {
    graph: ElkNodeRef,
    pretty_print: bool,
    omit_zero_position: bool,
    omit_zero_dimension: bool,
    omit_layout_information: bool,
    short_layout_option_keys: bool,
    omit_unknown_layout_options: bool,
}

impl ExportBuilder {
    fn new(graph: ElkNodeRef) -> Self {
        ExportBuilder {
            graph,
            pretty_print: false,
            omit_zero_position: true,
            omit_zero_dimension: true,
            omit_layout_information: false,
            short_layout_option_keys: true,
            omit_unknown_layout_options: true,
        }
    }

    pub fn pretty_print(mut self, pretty: bool) -> Self {
        self.pretty_print = pretty;
        self
    }

    pub fn short_layout_option_keys(mut self, short_keys: bool) -> Self {
        self.short_layout_option_keys = short_keys;
        self
    }

    pub fn omit_zero_positions(mut self, omit_zero_pos: bool) -> Self {
        self.omit_zero_position = omit_zero_pos;
        self
    }

    pub fn omit_zero_dimension(mut self, omit_zero_dim: bool) -> Self {
        self.omit_zero_dimension = omit_zero_dim;
        self
    }

    pub fn omit_layout(mut self, omit_layout: bool) -> Self {
        self.omit_layout_information = omit_layout;
        self
    }

    pub fn omit_unknown_layout_options(mut self, omit_unknown_options: bool) -> Self {
        self.omit_unknown_layout_options = omit_unknown_options;
        self
    }

    pub fn to_json(self) -> String {
        let mut exporter = JsonExporter::new();
        exporter.set_options(
            self.omit_zero_position,
            self.omit_zero_dimension,
            self.omit_layout_information,
            self.short_layout_option_keys,
            self.omit_unknown_layout_options,
        );
        let json_graph = exporter.export(&self.graph);

        if self.pretty_print {
            serde_json::to_string_pretty(&json_graph).unwrap_or_else(|_| "{}".to_string())
        } else {
            serde_json::to_string(&json_graph).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

enum GraphInput {
    Text(String),
    Value(Rc<RefCell<Value>>),
}

fn parse_lenient(text: &str) -> Result<Value, JsonImportError> {
    let sanitized = preprocess_lenient_json(text);
    json5::from_str(&sanitized)
        .map_err(|err| JsonImportError::from(JsonIOException::new(err.to_string())))
}

fn preprocess_lenient_json(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut escape = false;

    while let Some(ch) = chars.next() {
        if in_line_comment {
            out.push(ch);
            if ch == '\n' {
                in_line_comment = false;
            }
            continue;
        }

        if in_block_comment {
            out.push(ch);
            if ch == '*' && matches!(chars.peek(), Some('/')) {
                out.push('/');
                chars.next();
                in_block_comment = false;
            }
            continue;
        }

        if in_single {
            out.push(ch);
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        if in_double {
            out.push(ch);
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
            if ch == '"' {
                in_double = false;
            }
            continue;
        }

        if ch == '/' {
            if let Some(next) = chars.peek().copied() {
                if next == '/' {
                    in_line_comment = true;
                    out.push(ch);
                    out.push(next);
                    chars.next();
                    continue;
                }
                if next == '*' {
                    in_block_comment = true;
                    out.push(ch);
                    out.push(next);
                    chars.next();
                    continue;
                }
            }
        }

        if ch == '\'' {
            in_single = true;
            out.push(ch);
            continue;
        }
        if ch == '"' {
            in_double = true;
            out.push(ch);
            continue;
        }

        if ch == ';' {
            out.push(',');
            continue;
        }

        out.push(ch);
    }

    out
}
