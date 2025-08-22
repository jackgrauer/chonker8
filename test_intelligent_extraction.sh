#!/bin/bash

echo "=========================================="
echo "  CHONKER8 INTELLIGENT EXTRACTION TEST"
echo "=========================================="
echo ""
echo "This test demonstrates the complete integration of:"
echo "  • Document-agnostic page analysis"
echo "  • Intelligent extraction method routing"
echo "  • Quality validation with fallback"
echo "  • Hot-reload UI integration"
echo ""

# Find a test PDF
TEST_PDF=""
if [ -f "/Users/jack/Desktop/BERF-CERT.pdf" ]; then
    TEST_PDF="/Users/jack/Desktop/BERF-CERT.pdf"
elif [ -f "$1" ]; then
    TEST_PDF="$1"
else
    echo "Please provide a PDF path as argument"
    exit 1
fi

echo "Using PDF: $TEST_PDF"
echo ""

# Test 1: Direct extraction test
echo "1. Testing direct intelligent extraction:"
echo "-----------------------------------------"
DYLD_LIBRARY_PATH=./lib ./target/release/test-extraction analyze "$TEST_PDF" --page 0
echo ""

# Test 2: PDF processor with intelligent extraction
echo "2. Testing hot-reload processor with intelligent extraction:"
echo "------------------------------------------------------------"
DYLD_LIBRARY_PATH=./lib ./target/release/pdf-processor process "$TEST_PDF" 0
echo ""

# Test 3: Hot-reload UI
echo "3. Launching hot-reload UI with intelligent extraction:"
echo "--------------------------------------------------------"
echo "The UI will now show:"
echo "  • Left pane: PDF image"
echo "  • Right pane: Intelligently extracted text"
echo ""
echo "Press Ctrl+C to exit the UI"
echo "Press any key to launch..."
read -n 1

# Launch the hot-reload UI
DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot