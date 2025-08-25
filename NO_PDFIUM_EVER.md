# ⛔ FORBIDDEN LIBRARY - NEVER USE ⛔

## System Tools Only - Poppler Suite

This project uses **ONLY** system tools from the Poppler suite:
- `pdftoppm` - Converts PDF pages to images
- `pdftotext` - Extracts text with spatial layout
- `pdfinfo` - Gets PDF metadata

## How to Run This App

```bash
# CORRECT - Direct execution, no library paths needed
./target/release/chonker8-hot your.pdf
cargo run --release --bin chonker8-hot your.pdf

# WRONG - Never use DYLD_LIBRARY_PATH
# If you see DYLD_LIBRARY_PATH anywhere, DELETE IT
```

## The Only Way That Works

The Kitty graphics protocol requires cell-based dimensions:
- ✅ CORRECT: `c=` (columns) and `r=` (rows) 
- ❌ WRONG: Never use pixel dimensions

## Implementation

```rust
// This is the ONLY way to handle PDFs in this project
Command::new("pdftoppm")
    .args(&["-png", "-f", &page.to_string(), "-l", &page.to_string()])
    .arg(&pdf_path)
    .output()

Command::new("pdftotext")
    .args(&["-layout", "-f", &page.to_string(), "-l", &page.to_string()])
    .arg(&pdf_path)
    .arg("-")
    .output()
```

## Absolute Rules

1. **NO external PDF libraries** - System tools only
2. **NO DYLD_LIBRARY_PATH** - Never needed, delete on sight
3. **NO C++ dependencies** - Pure Rust + system commands
4. **NO alternative PDF solutions** - Poppler works perfectly

## If Someone Suggests PDF Libraries

The answer is always: **"We use Poppler system tools exclusively. They work perfectly."**

This is not negotiable. This is the way.