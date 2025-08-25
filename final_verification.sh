#!/bin/bash

echo "🎯 Final Verification of UTF-8 Fix in chonker8-hot"
echo "=================================================="
echo ""
echo "Binary: ./target/release/chonker8-hot"
echo "Built: $(ls -lh ./target/release/chonker8-hot | awk '{print $6, $7, $8, $9}')"
echo ""

# Check for the specific PDFs that were causing issues
echo "Checking for problematic PDFs in Desktop:"
ls /Users/jack/Desktop/*.pdf 2>/dev/null | while read -r pdf; do
    filename=$(basename "$pdf")
    if [[ "$filename" == *"é"* ]] || [[ "$filename" == *"–"* ]] || [[ "$filename" == *"—"* ]]; then
        echo "  Found: $filename"
    fi
done

echo ""
echo "Running chonker8-hot (press Ctrl+C to exit)..."
echo "If you see the file picker without a crash, the fix is working!"
echo ""

cd /Users/jack/chonker8
./target/release/chonker8-hot