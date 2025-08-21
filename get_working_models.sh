#!/bin/bash
set -e

echo "📦 Getting OAR-OCR compatible models..."

cd models/
rm -f *.onnx

# Based on the user's feedback, we need specific file sizes:
# - Detection: ~4.7MB
# - Recognition: ~10.8MB

echo "⬇️  Downloading from GitHub release assets..."

# Try RapidAI's release assets directly
curl -L -o ppocrv4_mobile_det.onnx \
    "https://github.com/RapidAI/RapidOCR/releases/download/v1.3.16/ch_PP-OCRv4_det_infer.onnx" \
    --progress-bar || echo "Failed det model"

curl -L -o ppocrv4_mobile_rec.onnx \
    "https://github.com/RapidAI/RapidOCR/releases/download/v1.3.16/ch_PP-OCRv4_rec_infer.onnx" \
    --progress-bar || echo "Failed rec model"

echo "✅ Final check:"
ls -lh *.onnx *.txt
