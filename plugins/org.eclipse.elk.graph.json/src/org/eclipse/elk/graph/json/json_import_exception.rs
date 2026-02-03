use std::error::Error;
use std::fmt;

#[derive(Clone, Debug)]
pub struct JsonImportException {
    message: String,
}

impl JsonImportException {
    pub fn new(message: impl Into<String>) -> Self {
        JsonImportException {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for JsonImportException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for JsonImportException {}

#[derive(Clone, Debug)]
pub struct JsonIOException {
    message: String,
}

impl JsonIOException {
    pub fn new(message: impl Into<String>) -> Self {
        JsonIOException {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for JsonIOException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for JsonIOException {}

#[derive(Clone, Debug)]
pub enum JsonImportError {
    Import(JsonImportException),
    Io(JsonIOException),
}

impl fmt::Display for JsonImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonImportError::Import(err) => write!(f, "{err}"),
            JsonImportError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl Error for JsonImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            JsonImportError::Import(err) => Some(err),
            JsonImportError::Io(err) => Some(err),
        }
    }
}

impl From<JsonImportException> for JsonImportError {
    fn from(err: JsonImportException) -> Self {
        JsonImportError::Import(err)
    }
}

impl From<JsonIOException> for JsonImportError {
    fn from(err: JsonIOException) -> Self {
        JsonImportError::Io(err)
    }
}
