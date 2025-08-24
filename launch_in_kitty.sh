#!/bin/bash

# Launch chonker8-hot in Kitty terminal for proper PDF display
PDF_FILE="${1:-/Users/jack/Desktop/BERF-CERT.pdf}"

echo "🚀 Launching A/B PDF Comparison Viewer in Kitty"
echo "================================================"
echo "Left Panel: PDF render via lopdf→vello→kitty"
echo "Right Panel: pdftotext extraction with layout"
echo ""
echo "PDF: $PDF_FILE"
echo ""

# Check if we're already in Kitty
if [ -n "$KITTY_WINDOW_ID" ]; then
    echo "✅ Already in Kitty! Launching directly..."
    ./target/release/chonker8-hot "$PDF_FILE"
else
    echo "🔄 Opening in Kitty terminal..."
    # Launch in Kitty
    if command -v kitty &> /dev/null; then
        kitty --title "chonker8 A/B PDF Viewer" \
              --override font_size=10 \
              ./target/release/chonker8-hot "$PDF_FILE"
    else
        echo "❌ Kitty terminal not found!"
        echo "Please install Kitty: https://sw.kovidgoyal.net/kitty/"
        echo ""
        echo "On macOS: brew install --cask kitty"
        exit 1
    fi
fi