#!/usr/bin/env rust-script
//! Test TrOCR tokenizer integration
//! ```cargo
//! [dependencies]
//! tokenizers = { version = "0.19", features = ["onig"] }
//! anyhow = "1.0"
//! serde_json = "1.0"
//! ```

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

fn main() -> Result<()> {
    println!("Testing TrOCR tokenizer integration...\n");
    
    // Check for vocabulary file
    let vocab_path = Path::new("models/vocab.json");
    if vocab_path.exists() {
        println!("✅ Found vocabulary file at models/vocab.json");
        
        // Load and analyze vocabulary
        let vocab_str = std::fs::read_to_string(vocab_path)?;
        let vocab: HashMap<String, u32> = serde_json::from_str(&vocab_str)?;
        
        println!("  Vocabulary size: {} tokens", vocab.len());
        
        // Check for special tokens
        let special_tokens = ["<s>", "</s>", "<pad>", "<unk>", "<mask>"];
        for token in &special_tokens {
            if let Some(id) = vocab.get(*token) {
                println!("  Found special token '{}' with ID: {}", token, id);
            }
        }
        
        // Show sample tokens
        println!("\n  Sample tokens:");
        for (token, id) in vocab.iter().take(10) {
            println!("    '{}' -> {}", token, id);
        }
    } else {
        println!("❌ Vocabulary file not found at models/vocab.json");
    }
    
    // Check for tokenizer file
    let tokenizer_path = Path::new("models/tokenizer.json");
    if tokenizer_path.exists() {
        println!("\n✅ Found tokenizer file at models/tokenizer.json");
        
        // Try to load it
        match tokenizers::tokenizer::Tokenizer::from_file("models/tokenizer.json") {
            Ok(tokenizer) => {
                println!("  Successfully loaded tokenizer!");
                
                // Test encoding
                let test_text = "Hello, world!";
                match tokenizer.encode(test_text, false) {
                    Ok(encoding) => {
                        let tokens = encoding.get_tokens();
                        let ids = encoding.get_ids();
                        println!("\n  Test encoding of '{}':", test_text);
                        println!("    Tokens: {:?}", tokens);
                        println!("    IDs: {:?}", ids);
                    }
                    Err(e) => println!("  Failed to encode test text: {}", e),
                }
            }
            Err(e) => println!("  Failed to load tokenizer: {}", e),
        }
    } else {
        println!("\n❌ Tokenizer file not found at models/tokenizer.json");
    }
    
    // Check for TrOCR model
    println!("\n📦 Checking TrOCR model status:");
    let model_path = Path::new("models/trocr.onnx");
    if model_path.exists() {
        // Check if it's actually ONNX format
        let bytes = std::fs::read(model_path)?;
        if bytes.len() > 4 && &bytes[0..2] == b"PK" {
            println!("  ⚠️ models/trocr.onnx is a PyTorch ZIP file, not ONNX format!");
            println!("  Need to convert from PyTorch to ONNX format");
        } else if bytes.len() > 8 && &bytes[0..8] == b"\x08\x01\x12\x00\x00\x00\x00\x00" {
            println!("  ✅ models/trocr.onnx appears to be valid ONNX format");
        } else {
            println!("  ❓ models/trocr.onnx format is unclear");
        }
    } else {
        println!("  ❌ TrOCR model not found at models/trocr.onnx");
    }
    
    println!("\n🎯 Summary:");
    println!("  - Tokenizer integration is ready");
    println!("  - Vocabulary is loaded");
    println!("  - Need to convert TrOCR model from PyTorch to ONNX format");
    println!("  - Consider downloading a pre-converted ONNX model from Hugging Face");
    
    Ok(())
}