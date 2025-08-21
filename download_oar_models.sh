#!/bin/bash
set -e

echo "📦 Downloading OAR-OCR models..."

# Create models directory
mkdir -p models

# Download detection model
echo "⬇️  Downloading detection model..."
curl -L https://github.com/RapidAI/RapidOCR/releases/download/v1.2.0/ppocrv4_mobile_det.onnx \
    -o models/ppocrv4_mobile_det.onnx \
    --progress-bar

# Download recognition model  
echo "⬇️  Downloading recognition model..."
curl -L https://github.com/RapidAI/RapidOCR/releases/download/v1.2.0/ppocrv4_mobile_rec.onnx \
    -o models/ppocrv4_mobile_rec.onnx \
    --progress-bar

# Download dictionary file
echo "⬇️  Downloading dictionary..."
curl -L https://raw.githubusercontent.com/PaddlePaddle/PaddleOCR/release/2.7/ppocr/utils/ppocr_keys_v1.txt \
    -o models/ppocr_keys_v1.txt \
    --progress-bar

echo "✅ OAR-OCR models downloaded successfully!"
ls -lh models/
