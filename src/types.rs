// Core types for Chonker8 CLI

// Error types
#[derive(Debug, thiserror::Error)]
pub enum ChonkerError {
    #[error("PDF error: {0}")]
    Pdf(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Extraction error: {0}")]
    Extraction(String),
}

pub type Result<T> = std::result::Result<T, ChonkerError>;