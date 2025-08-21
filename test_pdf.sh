#!/bin/bash

# Simple script to test chonker8 on any PDF

if [ $# -eq 0 ]; then
    echo "Usage: ./test_pdf.sh <pdf_file> [page_number]"
    echo ""
    echo "Examples:"
    echo "  ./test_pdf.sh document.pdf"
    echo "  ./test_pdf.sh document.pdf 2"
    echo "  ./test_pdf.sh document.pdf 1 --ocr"
    exit 1
fi

PDF="$1"
PAGE="${2:-1}"
OCR_FLAG=""

# Check if OCR flag is passed
if [[ "$*" == *"--ocr"* ]]; then
    OCR_FLAG="--mode ocr"
fi

if [ ! -f "$PDF" ]; then
    echo "Error: File '$PDF' not found"
    exit 1
fi

echo "================================================"
echo "🔍 CHONKER8 PDF ANALYZER"
echo "================================================"
echo ""
echo "📄 File: $(basename "$PDF")"
echo "📃 Page: $PAGE"
echo ""

# Extract and analyze
echo "🚀 Extracting text..."
echo "------------------------"
if [ -n "$OCR_FLAG" ]; then
    echo "Mode: OCR (Tesseract)"
    DYLD_LIBRARY_PATH=./lib ./target/release/chonker8 extract "$PDF" --page "$PAGE" --mode ocr --width 100 --height 30
else
    echo "Mode: Auto-detect"
    DYLD_LIBRARY_PATH=./lib ./target/release/chonker8 extract "$PDF" --page "$PAGE" --width 100 --height 30
fi

echo ""
echo "🧠 Document Analysis"
echo "------------------------"

# Get raw text for analysis
TEXT=$(DYLD_LIBRARY_PATH=./lib ./target/release/chonker8 extract "$PDF" --page "$PAGE" --raw 2>&1 | tail -n +6)

# Document type
echo -n "Type: "
if echo "$TEXT" | grep -qi "invoice\|bill\|total\|payment"; then
    echo "Invoice/Receipt"
elif echo "$TEXT" | grep -qi "certificate\|certify\|birth\|death"; then
    echo "Certificate"
elif echo "$TEXT" | grep -qi "dear\|sincerely\|regards"; then
    echo "Letter"
elif echo "$TEXT" | grep -qi "experience\|education\|skills\|resume"; then
    echo "Resume/CV"
elif echo "$TEXT" | grep -qi "contract\|agreement\|terms"; then
    echo "Contract/Agreement"
else
    echo "General Document"
fi

# Character count
CHARS=$(echo "$TEXT" | wc -c)
echo "Characters: $CHARS"

# Check if likely scanned
if [ "$CHARS" -lt 100 ]; then
    echo "Status: Likely scanned (try --ocr flag)"
else
    echo "Status: Text embedded"
fi

# Look for key elements
echo ""
echo "📊 Key Elements Found:"
if echo "$TEXT" | grep -qE "[A-Z][a-z]+ [0-9]{1,2}, [0-9]{4}|[0-9]{1,2}[/-][0-9]{1,2}[/-][0-9]{2,4}"; then
    echo "  ✓ Dates"
fi
if echo "$TEXT" | grep -qE "\\\$[0-9,]+\.?[0-9]*"; then
    echo "  ✓ Currency amounts"
fi
if echo "$TEXT" | grep -qE "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"; then
    echo "  ✓ Email addresses"
fi
if echo "$TEXT" | grep -qE "\([0-9]{3}\) [0-9]{3}-[0-9]{4}|[0-9]{3}-[0-9]{3}-[0-9]{4}"; then
    echo "  ✓ Phone numbers"
fi
if echo "$TEXT" | grep -qi "table\|header\|row\|column"; then
    echo "  ✓ Possible tables"
fi

echo ""
echo "------------------------"
echo "💡 Tips:"
echo "  • Use --ocr flag for scanned PDFs"
echo "  • Try different pages with second argument"
echo "  • Raw output: add --raw flag"