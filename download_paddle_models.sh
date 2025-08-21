#!/bin/bash
set -e

echo "📦 Downloading correct PP-OCRv4 models..."

cd models/

# Try another source - the official PaddleOCR converted models
echo "⬇️  Trying Hugging Face mirror for ONNX models..."

# Detection model
curl -L -o ch_PP-OCRv4_det_infer.onnx \
    "https://huggingface.co/spaces/K00B404/Rapid_OCR/resolve/main/models/ch_PP-OCRv4_det_infer.onnx" \
    --progress-bar

# Recognition model  
curl -L -o ch_PP-OCRv4_rec_infer.onnx \
    "https://huggingface.co/spaces/K00B404/Rapid_OCR/resolve/main/models/ch_PP-OCRv4_rec_infer.onnx" \
    --progress-bar

# Rename to match what OAR-OCR expects
if [ -f ch_PP-OCRv4_det_infer.onnx ]; then
    mv ch_PP-OCRv4_det_infer.onnx ppocrv4_mobile_det.onnx
fi

if [ -f ch_PP-OCRv4_rec_infer.onnx ]; then
    mv ch_PP-OCRv4_rec_infer.onnx ppocrv4_mobile_rec.onnx
fi

echo "✅ Checking downloaded models..."
ls -lh *.onnx 2>/dev/null || echo "No ONNX models found"
file *.onnx 2>/dev/null || true
