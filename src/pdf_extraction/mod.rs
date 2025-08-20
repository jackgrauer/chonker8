// PDF extraction module
pub mod basic;
pub mod improved;
pub mod true_visual;
pub mod ferrules_extraction;

pub use basic::{extract_to_matrix, get_page_count};
pub use true_visual::render_true_visual;
pub use ferrules_extraction::extract_with_ferrules;