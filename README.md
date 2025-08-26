# Chonker8 - Modern Terminal PDF Viewer

A high-performance terminal-based PDF viewer with split-screen display and advanced text extraction capabilities.

## Features

- **Split-Screen View**: PDF image on the left, extracted text on the right
- **Excel-Style Grid Editor**: Edit extracted text with full keyboard and mouse support
- **Dark Mode**: Automatic color inversion for terminal-friendly viewing
- **Search Functionality**: Fuzzy search with Ctrl+F powered by Nucleo
- **Mouse Support**: Click, drag, select, and edit with full mouse integration
- **Hot Reload**: Automatic UI refresh when PDF changes
- **Cross-Platform**: Works on macOS, Linux, and Windows
- **No PDFium Dependency**: Uses reliable system tools (pdftoppm, pdftotext)

## Installation

```bash
# Clone the repository
git clone https://github.com/jackgrauer/chonker8.git
cd chonker8

# Build the project
cargo build --release

# Run the viewer
./target/release/chonker8 path/to/your.pdf
```

## Requirements

- Rust 1.70 or later
- System tools:
  - `pdftoppm` (for PDF rendering)
  - `pdftotext` (for text extraction)
  
### Installing Dependencies

**macOS:**
```bash
brew install poppler
```

**Ubuntu/Debian:**
```bash
sudo apt-get install poppler-utils
```

**Arch Linux:**
```bash
sudo pacman -S poppler
```

## Usage

### Command Line

```bash
# Open a PDF file
./target/release/chonker8 document.pdf

# Run in test mode
./target/release/chonker8 --test-kitty

# Show version
./target/release/chonker8 --version
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

Chonker8 uses a clean, modular architecture:

```
chonker8/
├── src/
│   ├── main.rs              # Application entry point
│   ├── core/               # Core functionality
│   │   ├── config.rs       # Configuration management
│   │   └── hot_reload.rs   # File watching and hot reload
│   ├── display/            # UI components
│   │   ├── terminal_ui.rs  # Main terminal interface
│   │   ├── kitty_graphics.rs # Kitty terminal graphics
│   │   └── file_browser.rs # File selection dialog
│   └── pdf/                # PDF processing
│       ├── render_with_pdftoppm.rs # PDF to image
│       └── extract_text.rs # Text extraction with layout
└── Cargo.toml
```

### Key Components

- **PDF Rendering**: Uses system's `pdftoppm` for reliable PDF to image conversion
- **Text Extraction**: Enhanced `pdftotext` with spatial layout preservation
- **Terminal UI**: Crossterm for cross-platform terminal manipulation
- **Search Engine**: Nucleo for fuzzy text searching with highlighting
- **File Browser**: Integrated file picker with fuzzy search

## Version History

### v8.8.0 (Current)
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