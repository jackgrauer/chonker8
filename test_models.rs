#!/usr/bin/env rust-script
//! Test if models can be loaded

use ort::{init, Session};

fn main() {
    println!("Testing model loading...\n");
    
    // Initialize ORT
    let _ = init();
    
    // Test TrOCR
    println!("Testing TrOCR model:");
    match Session::builder()
        .unwrap()
        .commit_from_file("models/trocr.onnx") {
        Ok(_) => println!("  ✅ TrOCR loaded successfully"),
        Err(e) => println!("  ❌ TrOCR failed: {}", e),
    }
    
    // Test LayoutLM
    println!("\nTesting LayoutLM model:");
    match Session::builder()
        .unwrap()
        .commit_from_file("models/layoutlm.onnx") {
        Ok(_) => println!("  ✅ LayoutLM loaded successfully"),
        Err(e) => println!("  ❌ LayoutLM failed: {}", e),
    }
}