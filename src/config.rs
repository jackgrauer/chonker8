// Configuration constants for Chonker8
use std::env;
use std::path::PathBuf;

// Grid dimensions
pub const GRID_WIDTH: usize = 200;
pub const GRID_HEIGHT: usize = 100;

// Storage settings
pub const MAX_CACHED_PAGES: usize = 5;
pub const MAX_DEBUG_LOGS: usize = 1000;

// Get library path from environment or use default
pub fn pdfium_library_path() -> PathBuf {
    env::var("CHONKER_PDFIUM_PATH")
        .unwrap_or_else(|_| "./lib".to_string())
        .into()
}