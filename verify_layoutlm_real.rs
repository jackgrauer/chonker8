#!/usr/bin/env rust-script
//! Verify LayoutLM is actually processing data
//! ```cargo
//! [dependencies]
//! ort = { version = "2.0.0-rc.10", features = ["coreml"] }
//! anyhow = "1.0"
//! ```

use anyhow::Result;
use ort::{init, session::Session, value::Value, inputs};

fn main() -> Result<()> {
    println!("🔍 Verifying LayoutLMv3 Actually Works\n");
    
    let _ = init();
    
    // Load LayoutLM
    let mut layoutlm = Session::builder()?
        .commit_from_file("models/layoutlm.onnx")?;
    
    println!("Model info:");
    println!("  Inputs: {}", layoutlm.inputs.len());
    for (i, input) in layoutlm.inputs.iter().enumerate() {
        println!("    {}: {} - {:?}", i, input.name, input.input_type);
    }
    
    println!("\n  Outputs: {}", layoutlm.outputs.len());
    for (i, output) in layoutlm.outputs.iter().enumerate() {
        println!("    {}: {} - {:?}", i, output.name, output.output_type);
    }
    
    // Test with different input sizes
    println!("\nTesting with different sequence lengths:");
    
    for seq_len in [128, 256, 512] {
        println!("\n  Sequence length: {}", seq_len);
        
        // Prepare inputs
        let input_ids = Value::from_array(([1_usize, seq_len], vec![101i64; seq_len].into_boxed_slice()))?;
        let bbox = Value::from_array(([1_usize, seq_len, 4], vec![0i64; seq_len * 4].into_boxed_slice()))?;
        let attention_mask = Value::from_array(([1_usize, seq_len], vec![1i64; seq_len].into_boxed_slice()))?;
        let pixel_values = Value::from_array(([1_usize, 3, 224, 224], vec![0.5f32; 3 * 224 * 224].into_boxed_slice()))?;
        
        // Run inference
        match layoutlm.run(inputs![input_ids, bbox, attention_mask, pixel_values]) {
            Ok(outputs) => {
                let features = outputs[0].try_extract_tensor::<f32>()?;
                let (shape, data) = features;
                println!("    ✅ Output shape: {:?}", shape);
                
                // Check if data is non-zero (real processing)
                let non_zero_count = data.iter().filter(|&&x| x != 0.0).count();
                let total = data.len();
                let percent = (non_zero_count as f32 / total as f32) * 100.0;
                
                println!("    📊 Non-zero values: {}/{} ({:.1}%)", non_zero_count, total, percent);
                
                if percent > 90.0 {
                    println!("    ✅ Model is producing real outputs!");
                } else if percent > 50.0 {
                    println!("    ⚠️ Model output is partially sparse");
                } else {
                    println!("    ❌ Model output is mostly zeros - might not be working");
                }
            }
            Err(e) => {
                println!("    ❌ Inference failed: {}", e);
            }
        }
    }
    
    println!("\n🎯 Verification complete!");
    
    Ok(())
}