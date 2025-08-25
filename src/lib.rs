// Chonker8 - PDF viewer with A/B comparison
// Left panel: PDF rendered to image via pdftoppm
// Right panel: Text extracted via pdftotext

// Core modules
pub mod core {
    pub mod config;
    pub mod hot_reload;
}

// PDF processing - THIS IS WHAT WORKS!
pub mod pdf {
    pub mod render_with_pdftoppm;
    pub mod page_renderer;
    pub mod extract_text;
}

// Terminal display
pub mod display {
    pub mod kitty_graphics;
    pub mod kitty_helpers;
    pub mod ab_comparison_ui;
    pub mod terminal_ui;
    pub mod file_browser;
    pub mod theme;
}

// Machine learning extraction (future work)
pub mod ml_extraction;

// Storage
pub mod storage;

// Legacy compatibility exports (to avoid breaking everything at once)
pub use display::kitty_graphics as kitty_protocol;
pub use display::kitty_helpers as kitty_simple;
pub use display::ab_comparison_ui as enhanced_ab_ui;
pub use display::file_browser as integrated_file_picker;
pub use pdf::render_with_pdftoppm as system_pdf_renderer;
pub use pdf::page_renderer as pdf_renderer;
pub use pdf::extract_text as content_extractor;
pub use display::theme;