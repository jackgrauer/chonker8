#!/bin/bash

echo "Testing chonker8-hot PDF viewer arrow key fix..."
echo "================================"
echo ""
echo "This test will:"
echo "1. Launch chonker8-hot with a test PDF"
echo "2. You should see the PDF image on the left and text on the right"
echo "3. Press arrow keys - the PDF image should NOT disappear"
echo "4. The text on the right should be editable"
echo "5. Press Tab to switch screens, Esc to exit"
echo ""
echo "Press Enter to start the test..."
read

# Run the app with a test PDF
if [ -f "test.pdf" ]; then
    ./target/release/chonker8-hot test.pdf
else
    echo "No test.pdf found, running without PDF..."
    ./target/release/chonker8-hot
fi