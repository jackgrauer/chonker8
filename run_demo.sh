#!/bin/bash

echo "Starting Chonker8 Document-Agnostic PDF Extraction Demo"
echo "======================================================="
echo ""
echo "This demo shows a split-pane interface with:"
echo "  - Left pane: PDF visualization"
echo "  - Right pane: Intelligent extraction results"
echo ""
echo "The system automatically:"
echo "  • Analyzes page content (text/image coverage, quality)"
echo "  • Selects optimal extraction method"
echo "  • Validates quality and falls back if needed"
echo ""
echo "Controls:"
echo "  [n] - Next page"
echo "  [p] - Previous page"
echo "  [o] - Open PDF"
echo "  [q] - Quit"
echo ""
echo "Press any key to start..."
read -n 1

# Run the demo
./target/release/chonker8-demo