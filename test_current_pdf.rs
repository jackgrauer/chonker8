#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! anyhow = "1.0"
//! ```

use anyhow::Result;
use rexpect::spawn_bash;

fn main() -> Result<()> {
    println!("Testing current PDF rendering with PDFium...");
    
    // Test the PDF rendering
    println!("Starting chonker8-hot with PDF...");
    let mut session = spawn_bash(Some(5_000))?; // 5 seconds timeout
    
    // Set up environment and run chonker8-hot
    session.send_line("cd /Users/jack/chonker8")?;
    session.send_line("DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot real_test.pdf 2>&1")?;
    
    // Look for key debug messages
    println!("Checking PDF loading...");
    match session.exp_string("[DEBUG] load_pdf called with: real_test.pdf") {
        Ok(_) => println!("✓ PDF loading initiated"),
        Err(e) => println!("✗ PDF loading failed: {}", e),
    }
    
    match session.exp_string("[DEBUG] PDF page rendered") {
        Ok(_) => println!("✓ PDF page rendered"),
        Err(e) => println!("✗ PDF rendering failed: {}", e),
    }
    
    match session.exp_string("[DEBUG] Successfully displayed image with ID:") {
        Ok(_) => {
            println!("✓ Kitty protocol working!");
            println!("\n✅ PDF display pipeline is fully functional!");
        }
        Err(_) => {
            println!("⚠️  Kitty protocol not working, checking fallback...");
            match session.exp_string("PDF EXTRACTION METADATA") {
                Ok(_) => println!("✓ Text extraction working"),
                Err(_) => println!("✗ No output detected"),
            }
        }
    }
    
    // Clean exit
    session.send_control('c')?;
    
    println!("\n🎉 Test completed!");
    Ok(())
}