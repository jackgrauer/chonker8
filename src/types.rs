// Core types for Chonker8 CLI

// Error types
#[derive(Debug, thiserror::Error)]
pub enum ChonkerError {
    #[error("Processing error: {0}")]
    Processing(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, ChonkerError>;