#!/bin/bash
set -e

echo "📦 Downloading PP-OCR models from known working sources..."

# Clean up old files
rm -f models/*.onnx

# Try to download from rapidocr-onnx project which has working models
echo "⬇️  Downloading PP-OCRv4 detection model (4.5MB)..."
curl -L -f https://github.com/RapidAI/RapidOcrOnnx/releases/download/v1.3.24/ch_PP-OCRv4_det_infer.onnx \
    -o models/ppocrv4_mobile_det.onnx \
    --progress-bar \
    || curl -L -f https://github.com/RapidAI/RapidOCR/releases/download/python-v1.3.24/ch_PP-OCRv4_det_infer.onnx \
    -o models/ppocrv4_mobile_det.onnx \
    --progress-bar

echo "⬇️  Downloading PP-OCRv4 recognition model (10MB)..."
curl -L -f https://github.com/RapidAI/RapidOcrOnnx/releases/download/v1.3.24/ch_PP-OCRv4_rec_infer.onnx \
    -o models/ppocrv4_mobile_rec.onnx \
    --progress-bar \
    || curl -L -f https://github.com/RapidAI/RapidOCR/releases/download/python-v1.3.24/ch_PP-OCRv4_rec_infer.onnx \
    -o models/ppocrv4_mobile_rec.onnx \
    --progress-bar

echo "✅ Models downloaded! Final check..."
ls -lh models/
file models/*.onnx || true
