#!/bin/bash

echo "Testing text editor functionality interactively"
echo "=============================================="
echo ""
echo "Instructions:"
echo "1. Type some text to verify typing works"
echo "2. Hold SHIFT and use arrow keys to select text"
echo "3. Press Ctrl+C to copy selection"
echo "4. Move cursor and press Ctrl+V to paste"
echo "5. Press ESC to exit"
echo ""
echo "Starting in 3 seconds..."
sleep 3

DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot real_test.pdf