#!/bin/bash

echo "🧪 Direct test of chonker8-hot with Unicode filenames"
echo "======================================================"

# Create temp directory with problematic PDFs
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

# Create PDFs with Unicode names that could cause crashes
echo "%PDF-1.4" > "Test—with—em—dashes—that—will—be—truncated—at—various—points—in—the—display.pdf"
echo "%PDF-1.4" > "Japanese_テスト_ファイル_with_many_日本語_characters.pdf"
echo "%PDF-1.4" > "Mixed_中文_العربية_עברית_русский_test.pdf"
echo "%PDF-1.4" > "A City's Lost Identity_ An Analysis – Mediapolis.pdf"

echo "Created test PDFs in $TEMP_DIR"
ls -la *.pdf

echo ""
echo "Running chonker8-hot (will timeout after 1 second)..."
echo ""

# Run and capture output
OUTPUT=$(DYLD_LIBRARY_PATH=/Users/jack/chonker8/lib timeout 1 /Users/jack/chonker8/target/release/chonker8-hot 2>&1 || true)

# Check for UTF-8 panic
if echo "$OUTPUT" | grep -q "not a char boundary"; then
    echo "❌ UTF-8 PANIC DETECTED!"
    echo "$OUTPUT"
    exit 1
else
    echo "✅ No UTF-8 panic detected!"
    if echo "$OUTPUT" | grep -q "Chonker8"; then
        echo "✅ File picker started successfully"
    fi
    echo ""
    echo "Output preview:"
    echo "$OUTPUT" | head -10
fi

# Cleanup
cd /
rm -rf "$TEMP_DIR"

echo ""
echo "🎉 Test completed successfully!"