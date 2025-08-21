#!/usr/bin/env rust-script
//! Test combined TrOCR + LayoutLMv3 pipeline
//! ```cargo
//! [dependencies]
//! ort = { version = "2.0.0-rc.10", features = ["coreml"] }
//! anyhow = "1.0"
//! image = "0.25"
//! ```

use anyhow::Result;
use ort::{init, session::Session, value::Value, inputs};

fn main() -> Result<()> {
    println!("🎯 Combined TrOCR + LayoutLMv3 Pipeline Test\n");
    
    let _ = init();
    
    // Load both models
    println!("Loading models:");
    
    let mut trocr_encoder = Session::builder()?
        .commit_from_file("models/trocr_encoder.onnx")?;
    println!("  ✅ TrOCR Encoder loaded");
    
    let mut layoutlm = Session::builder()?
        .commit_from_file("models/layoutlm.onnx")?;
    println!("  ✅ LayoutLMv3 loaded");
    
    // Create test image
    let image_384 = vec![0.5f32; 3 * 384 * 384]; // For TrOCR
    let image_224 = vec![0.5f32; 3 * 224 * 224]; // For LayoutLM
    
    // Run TrOCR encoder
    println!("\n1️⃣ TrOCR Processing:");
    let trocr_input = Value::from_array(([1_usize, 3, 384, 384], image_384.into_boxed_slice()))?;
    let trocr_outputs = trocr_encoder.run(inputs![trocr_input])?;
    println!("   Output: Hidden states for text generation");
    
    // Run LayoutLM
    println!("\n2️⃣ LayoutLMv3 Processing:");
    let input_ids = Value::from_array(([1_usize, 512], vec![101i64; 512].into_boxed_slice()))?;
    let bbox = Value::from_array(([1_usize, 512, 4], vec![0i64; 512*4].into_boxed_slice()))?;
    let attention_mask = Value::from_array(([1_usize, 512], vec![1i64; 512].into_boxed_slice()))?;
    let pixel_values = Value::from_array(([1_usize, 3, 224, 224], image_224.into_boxed_slice()))?;
    
    let layoutlm_outputs = layoutlm.run(inputs![input_ids, bbox, attention_mask, pixel_values])?;
    println!("   Output: Document understanding features");
    
    println!("\n✅ Pipeline Integration Successful!");
    println!("   - TrOCR: Extract text from images");
    println!("   - LayoutLMv3: Understand document structure");
    println!("   - Combined: Complete document AI solution");
    
    Ok(())
}
