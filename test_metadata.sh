#!/bin/bash

echo "🧪 Testing PDF Extraction Metadata Display"
echo "=========================================="
echo ""

# Find a test PDF
TEST_PDF="/Users/jack/Desktop/BERF-CERT.pdf"

if [ ! -f "$TEST_PDF" ]; then
    echo "Creating test PDF..."
    TEST_PDF="/tmp/test_metadata.pdf"
    echo "%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /MediaBox [0 0 612 792] /Contents 4 0 R >>
endobj
4 0 obj
<< /Length 44 >>
stream
BT
/F1 12 Tf
100 700 Td
(Test PDF for Metadata Display) Tj
ET
endstream
endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000229 00000 n
trailer
<< /Size 5 /Root 1 0 R >>
startxref
344
%%EOF" > "$TEST_PDF"
fi

echo "Running chonker8-hot with test PDF..."
echo "Instructions:"
echo "1. Press Tab to cycle through screens"
echo "2. Look for 'Extracted Text' screen to see metadata"
echo "3. Press 'q' to quit"
echo ""
echo "Starting in 2 seconds..."
sleep 2

cd /Users/jack/chonker8
DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot "$TEST_PDF"