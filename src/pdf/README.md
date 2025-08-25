# PDF Module - THIS IS WHAT ACTUALLY WORKS!

This module handles PDF processing using system utilities from Poppler.

## Files:

- **render_with_pdftoppm.rs** - Renders PDF pages to PNG images using `pdftoppm` command
  - Just 68 lines of code that actually works!
  - Input: PDF file path + page number
  - Output: PNG image as DynamicImage
  
- **page_renderer.rs** - High-level wrapper for PDF rendering
  - Coordinates between render and page counting
  
- **extract_text.rs** - Extract text from PDFs using `pdftotext` command
  - Preserves layout for side-by-side comparison

## How it works:

```rust
// Instead of trying to parse PDFs ourselves, we just call:
Command::new("pdftoppm")
    .args(&["-png", pdf_path, "-"])
    .output()
```

This is why it actually works - we use battle-tested C++ tools that have been developed for 20+ years!