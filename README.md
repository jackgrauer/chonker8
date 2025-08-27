# Chonker8 - Modern Terminal PDF Viewer

A lightweight (~5,000 lines), high-performance terminal-based PDF viewer with split-screen display, hot-reload support, and advanced text extraction capabilities.

## Features

- **Split-Screen View**: PDF image on the left, extracted text on the right
- **Kitty Graphics Protocol**: Native image display in supported terminals
- **OCR Support**: Automatic OCR for scanned PDFs using Tesseract
- **Hot Reload**: Automatic rebuild and UI refresh during development (chonker8-hot)
- **Large PDF Support**: Handles massive PDFs with timeouts and proper error handling
- **File Browser**: Built-in fuzzy file picker for PDF selection
- **Excel-Style Grid**: Navigate extracted text in a grid format
- **Dark Mode**: Automatic color inversion for terminal-friendly viewing
- **Viewport Management**: Independent rendering regions prevent UI corruption
- **Cross-Platform**: Works on macOS, Linux, and Windows
- **No PDFium Dependency**: Uses reliable system tools (pdftoppm, pdftotext, pdfinfo)

## Installation

```bash
# Clone the repository
git clone https://github.com/jackgrauer/chonker8.git
cd chonker8

# Build the project (with pdfium library path on macOS)
DYLD_LIBRARY_PATH=./lib cargo build --release

# Run the standard viewer
./target/release/chonker8 path/to/your.pdf

# Or run the hot-reload version for development
./target/release/chonker8-hot path/to/your.pdf
```

## Requirements

- Rust 1.70 or later
- System tools:
  - `pdftoppm` (for PDF rendering)
  - `pdftotext` (for text extraction)
  - `pdfinfo` (for PDF metadata)
  - `tesseract` (optional, for OCR support)
  - `timeout` (for handling large files)
  
### Installing Dependencies

**macOS:**
```bash
brew install poppler tesseract
```

**Ubuntu/Debian:**
```bash
sudo apt-get install poppler-utils tesseract-ocr
```

**Arch Linux:**
```bash
sudo pacman -S poppler tesseract
```

## Usage

### Command Line

```bash
# Open a PDF file (standard version)
./target/release/chonker8 document.pdf

# Run hot-reload version (auto-rebuilds on code changes)
./target/release/chonker8-hot document.pdf

# Open file browser if no PDF specified
./target/release/chonker8-hot

# Debug output is written to:
cat /tmp/chonker8_debug.txt
```

### Keyboard Shortcuts

**Navigation:**
- `n` / `p` - Next/Previous page
- Arrow keys - Navigate text grid
- Page Up/Down - Scroll content
- Home/End - Jump to start/end of line
- Ctrl+Home/End - Jump to start/end of document

**Text Editing:**
- Type to edit text at cursor
- Shift + Arrow keys - Select text
- Ctrl+A - Select all
- Ctrl+C - Copy selection
- Ctrl+X - Cut selection
- Ctrl+V - Paste
- Backspace/Delete - Remove characters
- Enter - New line

**Search:**
- Ctrl+F or F3 - Open search
- Enter - Find next match
- Shift+Enter - Find previous match
- Ctrl+N - Next match
- Ctrl+Shift+N - Previous match
- Esc - Exit search mode

**Other:**
- `q` - Quit application
- `m` - Toggle display mode
- `w` - Toggle word wrap
- `r` - Reload current page
- `Tab` - Switch between screens
- Ctrl+S - Save edited text

### Mouse Support

- **Single Click** - Position cursor
- **Click and Drag** - Select text
- **Double Click** - Select word
- **Triple Click** - Select entire line
- **Right Click** - Context menu (where supported)

## Architecture

Chonker8 uses a clean, modular architecture (~5,000 lines of Rust):

```
chonker8/
├── src/
│   ├── main.rs                     # Application entry point
│   ├── main_hotreload.rs          # Hot-reload entry point
│   ├── lib.rs                     # Library exports
│   ├── core/                      # Core functionality
│   │   ├── config.rs              # Configuration management
│   │   └── hot_reload.rs          # File watching and auto-rebuild
│   ├── display/                   # UI components
│   │   ├── terminal_ui/           # Terminal interface
│   │   │   ├── mod.rs             # UI module exports
│   │   │   └── renderer.rs        # Main rendering engine
│   │   ├── viewport.rs            # Viewport management
│   │   ├── kitty_graphics.rs      # Kitty protocol implementation
│   │   ├── kitty_helpers.rs       # Kitty utilities
│   │   ├── file_browser.rs        # Fuzzy file picker
│   │   ├── theme.rs               # Color themes
│   │   └── mouse_zones.rs         # Mouse interaction zones
│   ├── pdf/                       # PDF processing
│   │   ├── render_with_pdftoppm.rs # PDF to image conversion
│   │   ├── page_renderer.rs       # Page rendering coordination
│   │   └── ocr.rs                 # OCR support for scanned PDFs
│   └── storage/                   # Data storage
│       ├── mod.rs                 # Storage exports
│       └── excel_grid.rs          # Grid data structure
├── lib/                           # PDFium libraries
└── Cargo.toml                     # Dependencies
```

### Key Components

- **PDF Rendering**: Uses system's `pdftoppm` with timeout protection for large files
- **Text Extraction**: `pdftotext` with automatic OCR fallback for scanned PDFs
- **Viewport System**: Independent rendering regions prevent UI corruption
- **Terminal UI**: Crossterm for cross-platform terminal manipulation
- **Kitty Graphics**: Native image display in Kitty terminals
- **File Browser**: Fuzzy file picker with Nucleo search
- **Hot Reload**: Automatic rebuild on source changes (chonker8-hot)

## Version History

### v8.9.0 (Current)
- Added viewport abstraction to prevent UI corruption
- Implemented timeout protection for large PDF files (700MB+)
- Fixed pdftoppm output file detection (supports 4-digit padding)
- Added OCR support with Tesseract for scanned PDFs
- Improved error handling with debug output to `/tmp/chonker8_debug.txt`
- Removed 1,177+ lines of dead code (ab_comparison modules, unused methods)
- Fixed arrow key navigation in PDF viewer
- Added hot-reload development mode (chonker8-hot)

### v8.8.0
- Complete removal of PDFium dependency
- Enhanced text extraction with word grouping
- Fuzzy search integration with Nucleo
- Improved dark mode rendering
- Fixed PDF containment and aspect ratio issues
- Removed unused ML extraction modules

### v8.6.0
- Replaced notcurses with crossterm
- Added Excel-style grid editor
- Improved mouse support
- Added hot reload functionality

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development

```bash
# Run in development mode
cargo run -- test.pdf

# Run tests
cargo test

# Build with optimizations
cargo build --release
```

## License

MIT License - See LICENSE file for details

## Author

Jack Grauer ([@jackgrauer](https://github.com/jackgrauer))

## Acknowledgments

- Poppler utilities for PDF processing
- Crossterm for terminal manipulation
- Nucleo for fuzzy search functionality