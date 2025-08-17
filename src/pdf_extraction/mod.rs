// PDF extraction module - choose between basic and AI-powered
pub mod basic;
pub mod ai_powered;

pub use basic::{extract_to_matrix, get_page_count};