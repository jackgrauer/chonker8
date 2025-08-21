#!/usr/bin/env python3
"""
Download pre-converted TrOCR ONNX model from Hugging Face
"""

import os
import requests
from pathlib import Path

def download_file(url, destination):
    """Download a file with progress indicator"""
    response = requests.get(url, stream=True)
    response.raise_for_status()
    
    total_size = int(response.headers.get('content-length', 0))
    block_size = 8192
    downloaded = 0
    
    with open(destination, 'wb') as f:
        for chunk in response.iter_content(chunk_size=block_size):
            if chunk:
                f.write(chunk)
                downloaded += len(chunk)
                if total_size > 0:
                    percent = (downloaded / total_size) * 100
                    print(f"\rDownloading: {percent:.1f}% ({downloaded}/{total_size} bytes)", end='')
    
    print()  # New line after download
    return destination

def main():
    print("🚀 TrOCR ONNX Model Downloader\n")
    
    # Create models directory if it doesn't exist
    models_dir = Path("models")
    models_dir.mkdir(exist_ok=True)
    
    # Backup existing file if present
    existing_model = models_dir / "trocr.onnx"
    if existing_model.exists():
        backup_path = models_dir / "trocr_pytorch.pth"
        print(f"📦 Backing up existing file to {backup_path}")
        os.rename(existing_model, backup_path)
    
    print("\n📥 Downloading TrOCR ONNX models from Hugging Face...\n")
    
    # Option 1: TrOCR Small (faster, smaller)
    print("Option 1: TrOCR Small (printed text, 244MB)")
    small_url = "https://huggingface.co/microsoft/trocr-small-printed/resolve/main/onnx/model.onnx"
    
    # Option 2: TrOCR Base (better accuracy)
    print("Option 2: TrOCR Base (handwritten text, 1.3GB)")
    base_url = "https://huggingface.co/microsoft/trocr-base-handwritten/resolve/main/onnx/model.onnx"
    
    # For now, let's try the small model first
    print("\n🎯 Downloading TrOCR Small model (good for printed text)...")
    
    try:
        # Try to download from a working ONNX export
        # Using a community ONNX export that should work
        onnx_url = "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/onnx/decoder_model_merged.onnx"
        
        print(f"Downloading from: {onnx_url}")
        dest_path = models_dir / "trocr.onnx"
        download_file(onnx_url, dest_path)
        
        print(f"\n✅ Successfully downloaded TrOCR ONNX model to {dest_path}")
        print(f"   File size: {dest_path.stat().st_size / 1024 / 1024:.1f} MB")
        
        # Also download the encoder if needed
        encoder_url = "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/onnx/encoder_model.onnx"
        encoder_path = models_dir / "trocr_encoder.onnx"
        
        print(f"\n📥 Downloading encoder model...")
        download_file(encoder_url, encoder_path)
        print(f"✅ Downloaded encoder to {encoder_path}")
        
    except Exception as e:
        print(f"\n❌ Failed to download: {e}")
        print("\nAlternative: You can manually download ONNX models from:")
        print("  - https://huggingface.co/Xenova/trocr-small-printed")
        print("  - https://huggingface.co/Xenova/trocr-base-handwritten")
        print("\nOr convert the PyTorch model using:")
        print("  python -m transformers.onnx --model=microsoft/trocr-base-printed models/")
        return 1
    
    print("\n🎉 Done! Now test with: cargo run --release --bin test_models")
    return 0

if __name__ == "__main__":
    exit(main())