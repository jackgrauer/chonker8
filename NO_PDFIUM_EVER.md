# ⛔ ABSOLUTELY NO PDFIUM ⛔

## DO NOT USE PDFIUM IN THIS PROJECT

**PDFium has been permanently removed from this codebase.**

### Why No PDFium?
1. **Build complexity** - Requires DYLD_LIBRARY_PATH and external C++ libraries
2. **Not needed** - We use system's `pdftoppm` from Poppler which works perfectly
3. **Proven solution** - pdftoppm has worked since we fixed Kitty protocol parameters

### What We Use Instead
- **PDF Rendering**: `pdftoppm` (part of Poppler suite, pre-installed on most systems)
- **Text Extraction**: `pdftotext` (also from Poppler)
- **Page Counting**: `pdfinfo` (also from Poppler) with lopdf fallback

### The Fix That Made Everything Work
The issue was never about PDF rendering - it was the Kitty graphics protocol parameters:
- ❌ WRONG: `s=/v=` (pixel dimensions)  
- ✅ CORRECT: `c=/r=` (cell dimensions)

### DO NOT:
- Add pdfium_render crate
- Add any DYLD_LIBRARY_PATH requirements
- Try to link C++ PDF libraries
- Attempt to render PDFs in Rust directly

### DO:
- Keep using pdftoppm via Command::new()
- Keep the SystemPdfRenderer 
- Use Poppler tools (pdftoppm, pdftotext, pdfinfo)

**This file exists to prevent PDFium from ever coming back.**