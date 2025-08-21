#!/bin/bash

# Set up environment
export DYLD_LIBRARY_PATH="/Users/jack/chonker8/lib"
CHONKER="/Users/jack/chonker8/target/release/chonker8"

echo "=========================================="
echo "CHONKER8 v8.8.0 - ALL PDFS TEST"
echo "All Improvements: --hybrid --preserve-tables --metadata"
echo "=========================================="
echo ""

# PDF 1: BERF Certificate
echo "=== PDF 1: BERF-CERT.pdf (Certificate with structured layout) ==="
$CHONKER extract \
    "/Users/jack/Desktop/BERF-CERT.pdf" \
    --page 1 \
    --mode ocr \
    --format text \
    --hybrid \
    --preserve-tables \
    --metadata \
    2>&1 | head -25
echo ""
echo "=========================================="
echo ""

# PDF 2: Newspaper article
echo "=== PDF 2: Testing_the_waters (Multi-column newspaper) ==="
$CHONKER extract \
    "/Users/jack/Desktop/Testing_the_waters_for_floating_class_7.5M___Philadelphia_Daily_News_PA___February_17_2025__pX10.pdf" \
    --page 1 \
    --mode ocr \
    --format text \
    --hybrid \
    --preserve-tables \
    --metadata \
    2>&1 | head -25
echo ""
echo "=========================================="
echo ""

# PDF 3: Pollution incident
echo "=== PDF 3: Pollution Incident S. 51st St. ==="
$CHONKER extract \
    "/Users/jack/Desktop/[External] Pollution Incident S. 51st St..pdf" \
    --page 1 \
    --mode ocr \
    --format text \
    --hybrid \
    --preserve-tables \
    --metadata \
    2>&1 | head -25
echo ""
echo "=========================================="
echo ""

# PDF 4: Follow up toxic spill
echo "=== PDF 4: Follow up toxic spill on Bartram's Mile ==="
$CHONKER extract \
    "/Users/jack/Desktop/_External_ Follow up to our call re_ toxic spill on Bartram''''s Mile.pdf" \
    --page 1 \
    --mode ocr \
    --format text \
    --hybrid \
    --preserve-tables \
    --metadata \
    2>&1 | head -25
echo ""
echo "=========================================="
echo ""

# PDF 5: Sports Franchise Game book
echo "=== PDF 5: The Sports Franchise Game (Book) ==="
$CHONKER extract \
    "/Users/jack/Documents/_OceanofPDF.com_The_Sports_Franchise_Game_-_Kenneth_L_Shropshire.pdf" \
    --page 1 \
    --mode ocr \
    --format text \
    --hybrid \
    --preserve-tables \
    --metadata \
    2>&1 | head -25
echo ""
echo "=========================================="
echo ""

# PDF 6: chonker_test.pdf
echo "=== PDF 6: chonker_test.pdf (Test document) ==="
$CHONKER extract \
    "/Users/jack/Documents/chonker_test.pdf" \
    --page 1 \
    --mode ocr \
    --format text \
    --hybrid \
    --preserve-tables \
    --metadata \
    2>&1 | head -25
echo ""
echo "=========================================="
echo ""

# PDF 7: test.pdf
echo "=== PDF 7: test.pdf (Another test document) ==="
$CHONKER extract \
    "/Users/jack/Documents/test.pdf" \
    --page 1 \
    --mode ocr \
    --format text \
    --hybrid \
    --preserve-tables \
    --metadata \
    2>&1 | head -25
echo ""
echo "=========================================="
echo ""

# PDF 8: Right to know request (if it exists)
echo "=== PDF 8: Right to Know Request Results ==="
if [ -f "/Users/jack/Desktop/righttoknowrequestresultsfortieriidataon4southwes.pdf" ]; then
    $CHONKER extract \
        "/Users/jack/Desktop/righttoknowrequestresultsfortieriidataon4southwes.pdf" \
        --page 1 \
        --mode ocr \
        --format text \
        --hybrid \
        --preserve-tables \
        --metadata \
        2>&1 | head -25
else
    echo "File not found, skipping..."
fi
echo ""
echo "=========================================="
echo ""

# PDF 9: Entry level welding resume
echo "=== PDF 9: Entry Level Welding Resume ==="
if [ -f "/Users/jack/Desktop/entry-level-welding-resume-example.pdf" ]; then
    $CHONKER extract \
        "/Users/jack/Desktop/entry-level-welding-resume-example.pdf" \
        --page 1 \
        --mode ocr \
        --format text \
        --hybrid \
        --preserve-tables \
        --metadata \
        2>&1 | head -25
else
    echo "File not found, skipping..."
fi
echo ""
echo "=========================================="
echo ""

# PDF 10: 1-MorrisonFinal.pdf from chonker5
echo "=== PDF 10: 1-MorrisonFinal.pdf ==="
if [ -f "/Users/jack/chonker5/1-MorrisonFinal.pdf" ]; then
    $CHONKER extract \
        "/Users/jack/chonker5/1-MorrisonFinal.pdf" \
        --page 1 \
        --mode ocr \
        --format text \
        --hybrid \
        --preserve-tables \
        --metadata \
        2>&1 | head -25
else
    echo "File not found, skipping..."
fi
echo ""

echo "=========================================="
echo "SUMMARY OF IMPROVEMENTS APPLIED:"
echo "=========================================="
echo ""
echo "✅ --hybrid: Mix OCR and pdftotext based on confidence"
echo "✅ --preserve-tables: Detect and preserve table structures"
echo "✅ --metadata: Include document metadata and encoding"
echo "✅ Dynamic grid sizing: Prevents any text truncation"
echo "✅ Better line grouping: 0.008 tolerance for accuracy"
echo ""