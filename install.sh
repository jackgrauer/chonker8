#!/bin/bash

# Install script to make chonker8 available system-wide

echo "🚀 Installing Chonker8 Hot-Reload TUI"
echo "====================================="

# Build the release version
echo "Building release version..."
cargo build --release --bin chonker8-hot

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

# Create a launcher script
echo "Creating launcher script..."
cat > chonker8 << 'EOF'
#!/bin/bash
# Chonker8 Hot-Reload TUI Launcher

CHONKER_HOME="$HOME/chonker8"

# Check if ui.toml exists in current directory, if not use the default
if [ ! -f "ui.toml" ]; then
    cp "$CHONKER_HOME/ui.toml" . 2>/dev/null || true
fi

# Run chonker8 with proper library path
DYLD_LIBRARY_PATH="$CHONKER_HOME/lib" "$CHONKER_HOME/target/release/chonker8-hot" "$@"
EOF

chmod +x chonker8

# Option 1: Install to /usr/local/bin (requires sudo)
echo ""
echo "Option 1: Install system-wide (requires sudo)"
echo "  sudo mv chonker8 /usr/local/bin/"
echo ""
echo "Option 2: Add to your PATH"
echo "  mkdir -p ~/.local/bin"
echo "  mv chonker8 ~/.local/bin/"
echo "  # Add this to your ~/.zshrc or ~/.bashrc:"
echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
echo ""

# Option 3: Just create an alias
echo "Option 3: Create an alias (add to ~/.zshrc or ~/.bashrc):"
echo "  alias chonker8='DYLD_LIBRARY_PATH=$HOME/chonker8/lib $HOME/chonker8/target/release/chonker8-hot'"
echo ""

echo "✅ Build complete! Choose an installation option above."