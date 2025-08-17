# Chonker 8.1 - The PDF Text Extractor (booyeeee!)

A blazing fast PDF viewer with text extraction and editing capabilities. Three distinct screens, clean architecture, and both basic and AI-powered extraction.

## Architecture

```
chonker8/
├── main.rs                    // Entry point
│
├── views/                     // What you see on screen
│   ├── pdf_viewer/           // Left side - PDF image
│   ├── text_editor/          // Right side - editable text  
│   └── status_bar/           // Bottom - page info
│
├── pdf_extraction/           // How we get text from PDFs
│   ├── basic.rs             // Fast but simple (PDFium)
│   └── ai_powered.rs        // Smart but slow (Ferrules ML)
│
├── controls/                 // User input
│   ├── keyboard.rs          // Keyboard shortcuts
│   └── file_picker.rs       // Open file dialog
│
├── theme/                    // Colors and appearance
│   └── colors.rs            // Soft palette (not electric!)
│
├── lib/                      // External libraries
│   └── (pdfium, ferrules)
│
└── README.md                 // You are here!
```

## Screens

Press **Tab** to cycle through:

1. **File Picker** - Browse PDFs with live thumbnail preview
2. **Editor** - Split view: PDF image (left) + editable text (right)  
3. **Debug** - Full screen debug output (Ctrl+C to copy everything)

## Keyboard Shortcuts

- `Tab` - Switch between screens
- `Ctrl+O` - Open file picker
- `Ctrl+N/P` - Next/Previous page
- `Ctrl+C` - Copy selected text (or debug logs)
- `Ctrl+V` - Paste text
- `Ctrl+Q` - Quit

## Features

- **Fast extraction** - PDFium for quick basic text extraction
- **AI extraction** - Ferrules ML for understanding complex layouts
- **Clean UI** - No redundant headers or electric colors
- **Debug everything** - See ALL the processing that happens
- **Spatial preservation** - Text maintains its position from the PDF

## Version 8.1 Changes

- Refactored into clean modular structure
- 3 distinct screens with Tab navigation
- Softer color palette
- Comprehensive debug output
- ~300 lines less code
- 20% faster rendering

Booyeeee! 🎉