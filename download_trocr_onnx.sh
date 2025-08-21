#!/bin/bash

echo "🚀 TrOCR ONNX Model Downloader"
echo ""

# Backup existing PyTorch model if present
if [ -f "models/trocr.onnx" ]; then
    echo "📦 Backing up existing PyTorch file to models/trocr_pytorch.pth"
    mv models/trocr.onnx models/trocr_pytorch.pth
fi

echo "📥 Downloading TrOCR ONNX model from Hugging Face..."
echo ""

# Download the decoder model (this is the main TrOCR model)
echo "Downloading TrOCR decoder model..."
curl -L -o models/trocr.onnx \
    "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/onnx/decoder_model_merged.onnx"

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Successfully downloaded TrOCR decoder model"
    
    # Download the encoder model
    echo ""
    echo "Downloading TrOCR encoder model..."
    curl -L -o models/trocr_encoder.onnx \
        "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/onnx/encoder_model.onnx"
    
    if [ $? -eq 0 ]; then
        echo ""
        echo "✅ Successfully downloaded TrOCR encoder model"
    else
        echo "⚠️ Failed to download encoder model"
    fi
    
    # Download tokenizer config if needed
    echo ""
    echo "Downloading tokenizer configuration..."
    curl -L -o models/tokenizer.json \
        "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/tokenizer.json"
    
    if [ $? -eq 0 ]; then
        echo "✅ Successfully downloaded tokenizer"
    fi
    
    echo ""
    echo "📊 Downloaded files:"
    ls -lh models/*.onnx models/tokenizer.json 2>/dev/null
    
    echo ""
    echo "🎉 Done! Now test with: cargo run --release --bin test_models"
else
    echo ""
    echo "❌ Failed to download TrOCR model"
    echo ""
    echo "Alternative: Manually download from:"
    echo "  https://huggingface.co/Xenova/trocr-small-printed/tree/main/onnx"
fi