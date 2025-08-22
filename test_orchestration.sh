#!/bin/bash

# Test orchestration script for document-agnostic PDF extraction

echo "Building test-extraction binary..."
DYLD_LIBRARY_PATH=./lib cargo build --release --bin test-extraction --quiet

if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo "Test binary built successfully!"
echo ""

# Create a sample test PDF if it doesn't exist
TEST_PDF="/tmp/test_document.pdf"

if [ ! -f "$TEST_PDF" ]; then
    echo "Creating test PDF..."
    # Try to find a PDF in common locations
    if [ -f "$HOME/Desktop/BERF-CERT.pdf" ]; then
        TEST_PDF="$HOME/Desktop/BERF-CERT.pdf"
    elif [ -f "$HOME/Documents/sample.pdf" ]; then
        TEST_PDF="$HOME/Documents/sample.pdf"
    else
        echo "Please provide a test PDF path as argument: $0 <pdf_path>"
        echo "Or place a PDF at /tmp/test_document.pdf"
        exit 1
    fi
fi

# Override with command line argument if provided
if [ "$1" != "" ]; then
    TEST_PDF="$1"
fi

echo "Using test PDF: $TEST_PDF"
echo "=================================================="
echo ""

# Test 1: Document Analysis
echo "TEST 1: Document Analysis"
echo "--------------------------"
DYLD_LIBRARY_PATH=./lib ./target/release/test-extraction analyze "$TEST_PDF" --page 0
echo ""

# Test 2: Automatic Extraction with Routing
echo "TEST 2: Automatic Extraction with Routing"
echo "------------------------------------------"
DYLD_LIBRARY_PATH=./lib ./target/release/test-extraction extract "$TEST_PDF" --page 0 --verbose
echo ""

# Test 3: Fallback Chain Testing
echo "TEST 3: Fallback Chain Testing"
echo "-------------------------------"
DYLD_LIBRARY_PATH=./lib ./target/release/test-extraction fallback "$TEST_PDF" --page 0
echo ""

# Test 4: Full Pipeline (multiple pages)
echo "TEST 4: Full Pipeline Test"
echo "---------------------------"
DYLD_LIBRARY_PATH=./lib ./target/release/test-extraction pipeline "$TEST_PDF"
echo ""

echo "=================================================="
echo "All tests completed!"