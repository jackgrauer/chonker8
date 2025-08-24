#!/bin/bash

# Test improved chonker8-hot with chunking and alignment fixes
echo "Testing improved chonker8-hot with chunking and alignment fixes..."
echo "Please run this in Kitty terminal!"
echo ""

# Test with a PDF
DYLD_LIBRARY_PATH=./lib timeout 30 ./target/release/chonker8-hot "/Users/jack/Desktop/BERF-CERT.pdf" 2>&1 | head -100

echo ""
echo "Check if:"
echo "1. Images are properly aligned (not shifted left)"
echo "2. No base64 text flooding the screen"
echo "3. Chunking works for large images"