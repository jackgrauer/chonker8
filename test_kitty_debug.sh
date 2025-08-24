#!/bin/bash

echo "Creating test PDF..."
echo "Test PDF" | ps2pdf - /tmp/test_kitty.pdf 2>/dev/null

echo "Running chonker8-hot with Kitty simulation..."
cd /Users/jack/chonker8

# Run in background and capture all output
export KITTY_WINDOW_ID=1
DYLD_LIBRARY_PATH=./lib timeout 2 ./target/release/chonker8-hot /tmp/test_kitty.pdf 2>&1 | tee /tmp/chonker8_debug.log &
PID=$!

sleep 1

echo "=== Debug Output ==="
cat /tmp/chonker8_debug.log | grep -E "DEBUG|Kitty|PDF|Display|image|Image" | head -30

# Clean up
kill $PID 2>/dev/null
wait $PID 2>/dev/null

echo ""
echo "=== Checking for key indicators ==="
if grep -q "PDF loaded successfully" /tmp/chonker8_debug.log; then
    echo "✓ PDF loaded"
else
    echo "✗ PDF not loaded"
fi

if grep -q "Kitty graphics protocol detected" /tmp/chonker8_debug.log; then
    echo "✓ Kitty detected"
else
    echo "✗ Kitty not detected"
fi

if grep -q "Screen::PdfViewer" /tmp/chonker8_debug.log; then
    echo "✓ Switched to PDF viewer screen"
else
    echo "✗ Did not switch to PDF viewer screen"
fi

if grep -q "PDF image size:" /tmp/chonker8_debug.log; then
    echo "✓ Image dimensions detected"
else
    echo "✗ Image dimensions not detected"
fi

if grep -q "Successfully displayed image" /tmp/chonker8_debug.log; then
    echo "✓ Image displayed via Kitty"
else
    echo "✗ Image not displayed"
fi