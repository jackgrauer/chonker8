#!/bin/bash

# Install chonker8 binaries to ~/.local/bin
mkdir -p ~/.local/bin

# Copy the main binaries
cp target/release/chonker8 ~/.local/bin/
cp target/release/chonker8-hot ~/.local/bin/
cp target/release/pdf-processor ~/.local/bin/

# Create a symlink so 'chonker8' launches the hot-reload TUI
ln -sf ~/.local/bin/chonker8-hot ~/.local/bin/chonker8-tui

echo "✅ chonker8 binaries installed to ~/.local/bin/"
echo ""
echo "Available commands:"
echo "  chonker8      - PDF extraction CLI"
echo "  chonker8-hot  - Hot-reload TUI (multi-screen with file picker)"
echo "  chonker8-tui  - Alias for chonker8-hot"
echo "  pdf-processor - Hot-reloadable PDF processor"
echo ""
echo "Make sure ~/.local/bin is in your PATH"
echo ""
echo "Add this to your .bashrc or .zshrc if needed:"
echo "export PATH=\"\$HOME/.local/bin:\$PATH\""