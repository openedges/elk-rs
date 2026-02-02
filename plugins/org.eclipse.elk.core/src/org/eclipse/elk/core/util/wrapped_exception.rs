use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct WrappedException {
    message: Option<String>,
    cause: Box<dyn Error + Send + Sync>,
}

impl WrappedException {
    pub fn new<E>(cause: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        WrappedException {
            message: None,
            cause: Box::new(cause),
        }
    }

    pub fn with_message<E>(message: impl Into<String>, cause: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        WrappedException {
            message: Some(message.into()),
            cause: Box::new(cause),
        }
    }

    pub fn cause(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.cause.as_ref()
    }
}

impl fmt::Display for WrappedException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.message {
            Some(message) => write!(f, "{message}: {}", self.cause),
            None => write!(f, "{}", self.cause),
        }
    }
}

impl Error for WrappedException {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}
