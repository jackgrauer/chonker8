#!/bin/bash

echo "Testing chonker8-hot with Vello PDF rendering..."

# Run chonker8-hot with timeout and capture output
timeout 3 ./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf 2>&1 | tee chonker8_hot_output.log | grep -E "\[DEBUG\]|\[INFO\]|\[VELLO\]|\[FONT\]|Rendering|PDF|Kitty" | head -30

echo ""
echo "Check chonker8_hot_output.log for full output"
echo "The PDF should be loaded and displayed if your terminal supports Kitty graphics"