# Chonker8-Hot Test Mode

## Overview
The test mode helps verify that the PDF viewer is working correctly, particularly after fixing issues with:
1. PDF images disappearing when arrow keys are pressed
2. Text extraction not displaying properly
3. Text editing functionality

## Running the Test

### Quick Start
```bash
# Run with automatic test mode
./run_test.sh

# Or directly with the binary
./target/release/chonker8-hot --test-ui

# With a specific PDF
./target/release/chonker8-hot --test-ui your_file.pdf
```

### What to Test

When the UI loads, verify these behaviors:

#### ✅ PDF Image Persistence
- Press arrow keys (↑ ↓ ← →)
- The PDF image on the left should remain visible
- Image should NOT disappear or flicker

#### ✅ Text Display
- Right panel should show extracted text
- Text should be clearly visible
- Metadata header shows extraction method and quality

#### ✅ Text Editing
- Arrow keys move the cursor in the text
- Typing replaces/inserts text at cursor
- Cursor is highlighted in yellow

#### ✅ Text Selection
- Hold Shift + Arrow keys to select text
- Selected text appears with blue background
- Type to replace selected block

#### ✅ File Operations
- Ctrl+S saves edited text to `[filename].edited.txt`
- Tab switches between screens
- Esc exits the application

## Test Checklist

After running the test, you should see a summary asking if these worked:

- [ ] PDF image remained visible when using arrow keys
- [ ] Text extraction displayed on the right panel
- [ ] Text cursor moved with arrow keys
- [ ] Text could be edited by typing
- [ ] Shift+arrows selected text blocks

## Troubleshooting

### No PDF Image
- Ensure you're using Kitty terminal or compatible
- Run `--test-kitty` to verify graphics support
- Check that the PDF file exists and is readable

### No Text Display
- Verify `pdftotext` is installed (`brew install poppler`)
- Check if the Excel grid is rendering (should show cursor)
- Look for extraction errors in terminal output

### Arrow Keys Not Working
- Ensure terminal is in raw mode (should be automatic)
- Check if the app is on the PDF viewer screen (not file picker)
- Try Tab to switch screens

## Implementation Details

The fixes implemented:
1. **Image persistence**: Removed the `image_sent` flag check so images are re-sent on each render
2. **Text display**: Changed from `render_text_extraction_panel()` to `render_text_content()` to use the Excel grid
3. **Unified data**: Excel grid now contains the extracted text and supports editing

## Files Modified
- `src/display/terminal_ui.rs`: Fixed image rendering and text display
- `src/main.rs`: Added test mode with helpful instructions