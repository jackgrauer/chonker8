# Chonker8 - PDF Viewer with A/B Comparison

## What This Does
Shows PDFs in terminal with side-by-side comparison:
- **Left panel**: PDF page as image (rendered via `pdftoppm`)
- **Right panel**: Extracted text (via `pdftotext`)

## How to Run
```bash
cargo build --release --bin chonker8-hot
./target/release/chonker8-hot
```

## Architecture That Actually Works

### The Secret: Don't Parse PDFs Yourself!
Instead of trying to implement PDF rendering (which we failed at with Vello, TinySkia, etc.), we just call system commands:

```rust
// Render PDF to image (68 lines of code)
Command::new("pdftoppm").args(&["-png", pdf_path]).output()

// Extract text
Command::new("pdftotext").args(&["-layout", pdf_path]).output()
```

### Module Structure

```
src/
├── main.rs                      # Entry point with hot-reload
├── pdf/                         # PDF processing (WORKING!)
│   ├── render_with_pdftoppm.rs # The 68 lines that actually work
│   └── extract_text.rs          # Text extraction
├── display/                     # Terminal UI
│   ├── kitty_graphics.rs       # Image display in terminal
│   └── ab_comparison_ui.rs     # Split-screen view
└── ml_extraction/              # Future ML work (LayoutLM, etc.)
```

### Key Files to Understand

1. **src/pdf/render_with_pdftoppm.rs** - The entire working PDF renderer in 68 lines
2. **src/display/kitty_graphics.rs** - How we show images in terminal (uses `c=` and `r=` for cells!)
3. **src/display/terminal_ui.rs** - Main UI loop and event handling

### What We Learned

- **Don't reinvent the wheel** - Poppler (pdftoppm/pdftotext) has 20+ years of development
- **Subprocess overhead is negligible** - 10ms to spawn process vs 200ms to render PDF
- **Simple is better** - 68 lines of working code beats 3000+ lines of broken renderers

## For Claude/LLMs Working on This

### Current Status
✅ PDF rendering works perfectly  
✅ Text extraction works perfectly  
✅ Kitty graphics display works  
✅ Hot reload works  

### Future Work
- LayoutLM integration for document understanding (in ml_extraction/)
- Better text selection in right panel
- Multiple page support with navigation

### Dependencies
- System: `pdftoppm`, `pdftotext` (from Poppler)
- Terminal: Kitty (for graphics support)
- Rust: See Cargo.toml

### Common Issues
- If images don't display: Check you're in Kitty terminal
- If PDFs don't render: Install poppler (`brew install poppler`)
- If build fails: We removed a lot of modules, check imports