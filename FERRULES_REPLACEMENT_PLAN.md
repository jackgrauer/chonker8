# Ferrules Replacement Plan

## Current Ferrules Usage
- OCR via macOS Vision API (unreliable, produces gibberish)
- Bounding box extraction (accurate but available elsewhere)
- Text block segmentation (good but not unique)

## Recommended Replacement Strategy

### Option 1: Pure PDFium (Already Available!)
```rust
// You already have pdfium-render = "0.8" 
// Use it directly for:
- Text extraction with positions
- Bounding boxes for layout preservation
- Page rendering for visual mode
- Direct PDF structure access
```

### Option 2: PDFium + Tesseract (For Real OCR)
```bash
# When OCR is actually needed:
brew install tesseract
# Then use tesseract-rs crate for OCR
```

### Option 3: Just pdftotext (Simplest)
```rust
// You already have this working perfectly!
// It handles 99% of PDFs correctly
pdf_extraction::extract_with_extractous_advanced()
```

## Implementation Steps

1. **Remove Ferrules Dependency**
   - Delete ferrules submodule
   - Remove ferrules_extraction.rs
   - Remove ferrules-core from Cargo.toml

2. **Replace with PDFium Direct**
   ```rust
   pub async fn extract_with_pdfium(
       pdf_path: &Path,
       page_index: usize,
   ) -> Result<Vec<Vec<char>>> {
       // Use existing pdfium-render to get text with positions
       // Already demonstrated in braille.rs and true_visual.rs!
   }
   ```

3. **Keep the Fallback System**
   - Try PDFium first
   - Fall back to pdftotext if needed
   - Add Tesseract only if real OCR is required

## Benefits of Dropping Ferrules

1. **Fewer dependencies** - No external binary to manage
2. **Better reliability** - PDFium is battle-tested
3. **Consistent quality** - No more gibberish OCR
4. **Simpler codebase** - Everything in Rust, no subprocesses
5. **Better performance** - No IPC overhead

## The Bottom Line

Ferrules adds complexity without unique value. Everything it does can be done better with tools you already have (PDFium) or standard tools (Tesseract for OCR, pdftotext for text extraction).