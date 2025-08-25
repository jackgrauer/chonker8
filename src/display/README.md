# Display Module

Terminal display using Kitty graphics protocol and crossterm.

## Files:

- **kitty_graphics.rs** - Kitty graphics protocol implementation
  - Sends PNG images to terminal using escape sequences
  - Key fix: Uses `c=` and `r=` for cell dimensions (not `s=` and `v=` for pixels!)

- **kitty_helpers.rs** - Simple helper functions for Kitty protocol

- **ab_comparison_ui.rs** - Split-screen A/B comparison view
  - Left panel: PDF page as image (from pdftoppm)
  - Right panel: Extracted text (from pdftotext)

- **terminal_ui.rs** - Main UI renderer and event handling
  - Manages different screens (file picker, PDF viewer, debug)
  - Handles keyboard/mouse input

- **file_browser.rs** - File picker with fuzzy search
  - Uses nucleo for fuzzy matching
  - Navigate filesystem to select PDFs

- **theme.rs** - Color schemes and styling