# Chonker8 Reorganization Plan for LLM Clarity

## New Directory Structure

```
src/
├── main.rs                       # Entry point for chonker8-hot
├── lib.rs                        # Library exports
│
├── core/                         # Core application logic
│   ├── app.rs                   # Main application state and lifecycle
│   ├── config.rs                # Configuration management
│   └── hot_reload.rs            # File watching and auto-rebuild
│
├── pdf/                          # PDF processing (WORKING!)
│   ├── render_with_pdftoppm.rs # Renders PDF to image using system pdftoppm
│   ├── extract_text.rs         # Extract text using pdftotext
│   └── page_info.rs            # Get page count using pdfinfo/lopdf
│
├── display/                      # Terminal display
│   ├── kitty_graphics.rs       # Kitty image protocol (WORKING!)
│   ├── ab_comparison_ui.rs     # Split view: PDF left, text right
│   ├── file_browser.rs         # File picker with fuzzy search
│   └── theme.rs                # Color schemes
│
├── ml_extraction/               # Machine learning extraction (FUTURE)
│   ├── layoutlm/               # Document understanding
│   ├── trocr/                  # OCR models
│   └── document_analyzer.rs    # Analysis pipeline
│
└── storage/                     # Data persistence
    └── sqlite_cache.rs         # Cache extracted text
```

## Renamed Files (Current → New)

- `main_hotreload.rs` → `main.rs`
- `system_pdf_renderer.rs` → `pdf/render_with_pdftoppm.rs`
- `pdf_renderer.rs` → `pdf/page_renderer.rs`
- `content_extractor.rs` → `pdf/extract_text.rs`
- `kitty_protocol.rs` → `display/kitty_graphics.rs`
- `kitty_simple.rs` → `display/kitty_helpers.rs`
- `enhanced_ab_ui.rs` → `display/ab_comparison_ui.rs`
- `integrated_file_picker.rs` → `display/file_browser.rs`
- `ui_renderer.rs` → `display/terminal_ui.rs`
- `ui_config.rs` → `core/config.rs`
- `hot_reload_manager.rs` → `core/hot_reload.rs`

## Why This is Better for LLMs

1. **Clear module boundaries** - Each directory has ONE clear purpose
2. **Descriptive names** - No ambiguity about what each file does
3. **Working vs Future** - Clear separation of working code vs ML experiments
4. **Flat where possible** - Less nesting = easier to navigate
5. **Comments at boundaries** - Each module will have a README explaining its purpose