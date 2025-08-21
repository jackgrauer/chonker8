#!/bin/bash
set -e

echo "📦 Downloading OAR-OCR models from HuggingFace..."

# Download from HuggingFace SWHL/RapidOCR repository
echo "⬇️  Downloading PP-OCRv4 detection model..."
curl -L https://huggingface.co/SWHL/RapidOCR/resolve/main/models/ch_PP-OCRv4_det_infer.onnx \
    -o models/ppocrv4_mobile_det.onnx \
    --progress-bar

echo "⬇️  Downloading PP-OCRv4 recognition model..."
curl -L https://huggingface.co/SWHL/RapidOCR/resolve/main/models/ch_PP-OCRv4_rec_infer.onnx \
    -o models/ppocrv4_mobile_rec.onnx \
    --progress-bar

echo "⬇️  Downloading dictionary..."
curl -L https://raw.githubusercontent.com/PaddlePaddle/PaddleOCR/release/2.7/ppocr/utils/ppocr_keys_v1.txt \
    -o models/ppocr_keys_v1.txt \
    --progress-bar

echo "✅ Models downloaded! Checking sizes..."
ls -lh models/
