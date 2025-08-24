#!/bin/bash

# Test the A/B viewer with Kitty mode
PDF="${1:-/Users/jack/Desktop/BERF-CERT.pdf}"

echo "🔍 Testing A/B PDF Viewer (Kitty Mode)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Simulate Kitty environment
export KITTY_WINDOW_ID=1
export TERM=xterm-kitty

echo "✅ Simulating Kitty environment:"
echo "   KITTY_WINDOW_ID=$KITTY_WINDOW_ID"
echo "   TERM=$TERM"
echo ""

# Run the viewer
echo "📄 Loading: $(basename "$PDF")"
echo ""

# Use script to capture terminal output properly
script -q /dev/null ./target/release/chonker8-hot "$PDF" 2>&1 | head -30

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✨ To see the full UI, run in actual Kitty:"
echo "   /Applications/kitty.app/Contents/MacOS/kitty ./target/release/chonker8-hot $PDF"