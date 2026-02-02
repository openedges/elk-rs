use std::fmt;

#[derive(Clone, Debug)]
pub struct UnsupportedConfigurationException {
    message: String,
}

impl UnsupportedConfigurationException {
    pub fn new(message: impl Into<String>) -> Self {
        UnsupportedConfigurationException {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for UnsupportedConfigurationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for UnsupportedConfigurationException {}
