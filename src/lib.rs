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
    pub mod ocr;
}

// Terminal display
pub mod display {
    pub mod kitty_graphics;
    pub mod kitty_helpers;
    pub mod terminal_ui;
    pub mod file_browser;
    pub mod theme;
    pub mod mouse_zones;
    pub mod viewport;
    pub mod debug_viewer;
}

// Storage
pub mod storage;

// Legacy compatibility exports (to avoid breaking everything at once)
pub use display::kitty_graphics as kitty_protocol;
pub use display::kitty_helpers as kitty_simple;
pub use display::file_browser as integrated_file_picker;
pub use pdf::render_with_pdftoppm as system_pdf_renderer;
pub use pdf::page_renderer as pdf_renderer;
pub use display::theme;