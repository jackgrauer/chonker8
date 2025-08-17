// Content extraction module - wrapper for PDF extraction methods
use anyhow::Result;
use std::path::Path;

// Re-export from pdf_extraction module
pub use crate::pdf_extraction::{extract_to_matrix, get_page_count};