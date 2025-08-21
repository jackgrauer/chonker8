# Replace Ferrules OCR with Tesseract

## The Problem
Ferrules uses macOS Vision API which is terrible for technical documents:
- Designed for photos/natural scenes, not documents
- No configuration options for document types
- Hardcoded in Ferrules source code
- Produces gibberish on tables and technical text

## The Solution: Tesseract OCR

### Install Tesseract
```bash
brew install tesseract
```

### Replace Ferrules OCR with Tesseract
```rust
use std::process::Command;

pub async fn extract_with_tesseract(
    pdf_path: &Path,
    page_index: usize,
) -> Result<Vec<Vec<char>>> {
    // Step 1: Convert PDF page to image
    let output = Command::new("pdftoppm")
        .args(&[
            "-f", &(page_index + 1).to_string(),
            "-l", &(page_index + 1).to_string(),
            "-png",
            "-singlefile",
            pdf_path.to_str().unwrap(),
            "/tmp/page"
        ])
        .output()?;
    
    // Step 2: Run Tesseract OCR
    let output = Command::new("tesseract")
        .args(&[
            "/tmp/page.png",
            "/tmp/ocr_output",
            "--psm", "6",  // Uniform block of text
            "-l", "eng",   // English
            "--oem", "3",  // Default OCR Engine Mode
            "tsv"          // Tab-separated output with positions
        ])
        .output()?;
    
    // Step 3: Parse TSV output with bounding boxes
    // Tesseract gives confidence scores and exact positions!
}
```

## Why Tesseract is Better

1. **Designed for documents** - Not photos
2. **Confidence scores** - Know when OCR is failing
3. **Multiple languages** - Can specify technical/equation modes
4. **Page segmentation modes** - Different modes for tables, columns, etc.
5. **Training data** - Can be trained on specific document types

## Page Segmentation Modes (--psm)
- 0 = Orientation and script detection only
- 1 = Automatic page segmentation with OSD
- 3 = Fully automatic (default)
- 6 = Uniform block of text
- 7 = Single text line
- 8 = Single word
- 9 = Single word in circle
- 10 = Single character
- 11 = Sparse text
- 12 = Sparse text with OSD
- 13 = Raw line (bypass heuristics)

For tables, use `--psm 6` or `--psm 11`.