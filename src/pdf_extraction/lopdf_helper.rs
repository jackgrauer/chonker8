// lopdf helper - Pure Rust PDF operations
use anyhow::Result;
use lopdf::Document;
use std::path::Path;

/// Load a PDF document using lopdf
pub fn load_pdf(path: &Path) -> Result<Document> {
    Ok(Document::load(path)?)
}

/// Execute an operation with a PDF document
pub fn with_pdf<F, R>(path: &Path, f: F) -> Result<R>
where
    F: FnOnce(&Document) -> Result<R>,
{
    let document = load_pdf(path)?;
    f(&document)
}