// PDF extraction module
pub mod basic;
pub mod improved;
pub mod true_visual;
// pub mod ferrules_extraction;  // FIRED - replaced with oar_extraction
pub mod oar_extraction;
pub mod extractous_extraction;
pub mod hybrid_extraction;

pub use basic::{extract_to_matrix, get_page_count};
pub use true_visual::render_true_visual;
// pub use ferrules_extraction::extract_with_ferrules;  // FIRED
pub use oar_extraction::extract_with_oar;
pub use extractous_extraction::{extract_with_extractous, extract_with_extractous_advanced};
pub use hybrid_extraction::extract_hybrid;