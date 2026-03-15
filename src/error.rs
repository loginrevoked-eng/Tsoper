use std::io;

#[derive(thiserror::Error, Debug)]
pub enum DowmanError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Registry error: {0}")]
    Registry(io::Error),
    
    #[error("{0}")]
    Other(String),
}

impl From<String> for DowmanError {
    fn from(err: String) -> Self {
        Self::Other(err)
    }
}

impl From<&str> for DowmanError {
    fn from(err: &str) -> Self {
        Self::Other(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, DowmanError>;
