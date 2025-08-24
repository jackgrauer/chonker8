#!/bin/bash

# Direct test of Kitty graphics with the Vello-rendered PDF

echo "Testing direct Kitty graphics display of Vello-rendered PDF..."

# First render the PDF with Vello
cargo run --release --bin test_vello 2>/dev/null

# Check if image was created
if [ -f "vello_render_test.png" ]; then
    echo "✓ Vello rendered the PDF to vello_render_test.png"
    
    # Try to display it with Kitty graphics protocol
    # This uses the icat command if available (comes with Kitty)
    if command -v icat &> /dev/null; then
        echo "✓ icat command found, displaying image..."
        icat vello_render_test.png
    else
        echo "✗ icat not found. Try running in Kitty terminal."
        echo "  The rendered PDF is saved as vello_render_test.png"
    fi
else
    echo "✗ Failed to render PDF"
fi