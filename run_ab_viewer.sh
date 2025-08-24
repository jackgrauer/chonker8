#!/bin/bash

# Perfect A/B PDF Comparison Viewer Launcher
# No fallbacks - Kitty graphics ONLY!

PDF="${1:-/Users/jack/Desktop/BERF-CERT.pdf}"

echo "🚀 Launching chonker8 A/B PDF Comparison Viewer"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📄 PDF: $(basename "$PDF")"
echo ""
echo "┌─────────────────────────┬─────────────────────────┐"
echo "│   LEFT: PDF RENDER      │   RIGHT: TEXT EXTRACT   │"
echo "│   lopdf→vello→kitty     │   pdftotext --layout    │"
echo "└─────────────────────────┴─────────────────────────┘"
echo ""

# Use the Kitty app directly
KITTY="/Applications/kitty.app/Contents/MacOS/kitty"

if [ -f "$KITTY" ]; then
    echo "✅ Kitty found at: $KITTY"
    echo "🖼️  Opening PDF viewer in Kitty terminal..."
    
    # Launch in a new Kitty window with optimal settings
    "$KITTY" \
        --single-instance \
        --title "chonker8 A/B PDF Viewer" \
        --override font_size=10 \
        --override initial_window_width=1600 \
        --override initial_window_height=900 \
        --override remember_window_size=no \
        bash -c "cd $(pwd) && ./target/release/chonker8-hot '$PDF'"
else
    echo "❌ Kitty not found at expected location"
    echo "Trying system kitty..."
    kitty ./target/release/chonker8-hot "$PDF"
fi