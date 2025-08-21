#!/bin/bash
set -e

PDF="/Users/jack/Desktop/Testing_the_waters_for_floating_class_7.5M___Philadelphia_Daily_News_PA___February_17_2025__pX10.pdf"
MODELS_DIR="/Users/jack/chonker8/models"
OUTPUT_DIR="/tmp/ocr_test_results"

mkdir -p "$OUTPUT_DIR"

echo "🧪 Testing all OCR models on newspaper PDF..."
echo "================================================"

# Test 1: Current Chinese/Multi mobile models
echo -e "\n1️⃣ PP-OCRv4 Chinese/Multi (Mobile - 4.5MB + 10MB):"
echo "---------------------------------------------------"
cd /Users/jack/chonker8
DYLD_LIBRARY_PATH=./lib ./target/release/chonker8 extract "$PDF" --page 1 --mode ocr --format text 2>&1 | tee "$OUTPUT_DIR/chinese_mobile.txt" | head -30

# Test 2: English models
echo -e "\n2️⃣ PP-OCRv3 English (2.3MB + 8.6MB):"
echo "--------------------------------------"
cd "$MODELS_DIR"
mv ppocrv4_mobile_det.onnx ppocrv4_mobile_det.onnx.bak
mv ppocrv4_mobile_rec.onnx ppocrv4_mobile_rec.onnx.bak
cp en_PP-OCRv3_det_infer.onnx ppocrv4_mobile_det.onnx
cp en_PP-OCRv3_rec_infer.onnx ppocrv4_mobile_rec.onnx
cd /Users/jack/chonker8
DYLD_LIBRARY_PATH=./lib ./target/release/chonker8 extract "$PDF" --page 1 --mode ocr --format text 2>&1 | tee "$OUTPUT_DIR/english.txt" | head -30

# Test 3: Chinese server models (high accuracy)
echo -e "\n3️⃣ PP-OCRv4 Chinese/Multi (Server - 108MB + 86MB):"
echo "----------------------------------------------------"
cd "$MODELS_DIR"
rm ppocrv4_mobile_det.onnx ppocrv4_mobile_rec.onnx
cp ch_PP-OCRv4_det_server_infer.onnx ppocrv4_mobile_det.onnx
cp ch_PP-OCRv4_rec_server_infer.onnx ppocrv4_mobile_rec.onnx
cd /Users/jack/chonker8
DYLD_LIBRARY_PATH=./lib ./target/release/chonker8 extract "$PDF" --page 1 --mode ocr --format text 2>&1 | tee "$OUTPUT_DIR/chinese_server.txt" | head -30

# Test 4: Native PDFium extraction
echo -e "\n4️⃣ Native PDFium extraction:"
echo "-----------------------------"
DYLD_LIBRARY_PATH=./lib ./target/release/chonker8 extract "$PDF" --page 1 --mode native --format text 2>&1 | tee "$OUTPUT_DIR/native.txt" | head -30

# Test 5: pdftotext (if OCR fails)
echo -e "\n5️⃣ Direct pdftotext:"
echo "--------------------"
pdftotext "$PDF" - | head -30 | tee "$OUTPUT_DIR/pdftotext.txt"

# Restore original models
echo -e "\n♻️ Restoring original models..."
cd "$MODELS_DIR"
rm -f ppocrv4_mobile_det.onnx ppocrv4_mobile_rec.onnx
mv ppocrv4_mobile_det.onnx.bak ppocrv4_mobile_det.onnx
mv ppocrv4_mobile_rec.onnx.bak ppocrv4_mobile_rec.onnx

echo -e "\n✅ All tests complete! Results saved in $OUTPUT_DIR"
echo "Files created:"
ls -lh "$OUTPUT_DIR"/*.txt