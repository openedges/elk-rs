use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct UnsupportedGraphException {
    message: String,
    cause: Option<Box<dyn Error + Send + Sync>>,
}

impl UnsupportedGraphException {
    pub fn new(message: impl Into<String>) -> Self {
        UnsupportedGraphException {
            message: message.into(),
            cause: None,
        }
    }

    pub fn with_cause(message: impl Into<String>, cause: impl Error + Send + Sync + 'static) -> Self {
        UnsupportedGraphException {
            message: message.into(),
            cause: Some(Box::new(cause)),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn cause(&self) -> Option<&(dyn Error + Send + Sync + 'static)> {
        self.cause.as_deref()
    }
}

impl fmt::Display for UnsupportedGraphException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for UnsupportedGraphException {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_deref().map(|cause| cause as &dyn Error)
    }
}
