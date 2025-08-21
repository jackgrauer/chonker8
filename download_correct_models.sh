#!/bin/bash
set -e

echo "📦 Downloading correct PP-OCRv4 models..."

cd models/

# Clean up any dummy files
rm -f *.onnx 2>/dev/null || true

# Download PP-OCRv4 detection model
echo "⬇️  Downloading PP-OCRv4 detection model..."
wget -q https://paddleocr.bj.bcebos.com/PP-OCRv4/chinese/ch_PP-OCRv4_det_infer.tar
tar -xf ch_PP-OCRv4_det_infer.tar
rm ch_PP-OCRv4_det_infer.tar

# Download PP-OCRv4 recognition model  
echo "⬇️  Downloading PP-OCRv4 recognition model..."
wget -q https://paddleocr.bj.bcebos.com/PP-OCRv4/chinese/ch_PP-OCRv4_rec_infer.tar
tar -xf ch_PP-OCRv4_rec_infer.tar
rm ch_PP-OCRv4_rec_infer.tar

echo "📝 Downloaded PaddleOCR models. These need to be converted to ONNX format."
echo "For now, let's try pre-converted ONNX models from RapidOCR..."

# Try to get pre-converted ONNX models from a working source
echo "⬇️  Downloading pre-converted ONNX models..."

# These are the correct URLs for pre-converted models
curl -L -o ppocrv4_mobile_det.onnx \
    https://github.com/RapidAI/RapidOCR-Miniprogram/releases/download/v1.0.0/ch_PP-OCRv4_det_infer.onnx \
    2>/dev/null || echo "Detection model download failed"

curl -L -o ppocrv4_mobile_rec.onnx \
    https://github.com/RapidAI/RapidOCR-Miniprogram/releases/download/v1.0.0/ch_PP-OCRv4_rec_infer.onnx \
    2>/dev/null || echo "Recognition model download failed"

echo "✅ Checking downloaded models..."
ls -lh *.onnx 2>/dev/null || echo "No ONNX models found yet"
ls -lh */inference.pdmodel 2>/dev/null || echo "PaddleOCR models need conversion"
