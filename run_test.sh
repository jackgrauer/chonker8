#!/bin/bash

echo "Running Chonker8-Hot UI Test Mode"
echo "=================================="
echo ""

# Check if we have a test PDF
if [ -f "test.pdf" ]; then
    echo "✓ Found test.pdf"
elif [ -f "real_test.pdf" ]; then
    echo "✓ Found real_test.pdf"
else
    echo "⚠ No test PDF found in current directory"
    echo "You can still test with the file picker or pass a PDF as argument"
fi

echo ""
echo "Starting test mode..."
echo ""

# Run with test mode flag
./target/release/chonker8-hot --test-ui "$@"