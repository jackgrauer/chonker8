#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! anyhow = "1.0"
//! ```

use anyhow::Result;
use rexpect::spawn_bash;

fn main() -> Result<()> {
    println!("Testing PDF rendering with lopdf + pathfinder...");
    
    // Build the project first
    println!("Building chonker8-hot...");
    let mut build = spawn_bash(Some(60_000))?; // 60 seconds timeout in milliseconds
    build.send_line("cd /Users/jack/chonker8")?;
    build.send_line("DYLD_LIBRARY_PATH=./lib cargo build --release --bin chonker8-hot --quiet")?;
    build.exp_string("warning:")?; // Wait for build to complete (warnings are expected)
    
    // Now test the PDF rendering
    println!("\nTesting PDF display...");
    let mut session = spawn_bash(Some(10_000))?; // 10 seconds timeout
    
    // Set up environment and run chonker8-hot
    session.send_line("cd /Users/jack/chonker8")?;
    session.send_line("DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot real_test.pdf")?;
    
    // Look for debug messages that indicate rendering is working
    println!("Waiting for PDF load...");
    session.exp_string("[DEBUG] load_pdf called with: real_test.pdf")?;
    println!("✓ PDF loading initiated");
    
    session.exp_string("[DEBUG] Page count:")?;
    println!("✓ Page count retrieved");
    
    session.exp_string("[DEBUG] Rendering PDF page")?;
    println!("✓ PDF rendering started");
    
    session.exp_string("[DEBUG] PDF page rendered")?;
    println!("✓ PDF page rendered successfully");
    
    session.exp_string("[DEBUG] Displaying PDF image at")?;
    println!("✓ Image display initiated");
    
    session.exp_string("[DEBUG] PDF image size:")?;
    println!("✓ Image dimensions calculated");
    
    session.exp_string("[DEBUG] Display size:")?;
    println!("✓ Display scaling calculated");
    
    // Check if Kitty protocol is working
    match session.exp_string("[DEBUG] Successfully displayed image with ID:") {
        Ok(_) => {
            println!("✓ Kitty protocol: Image displayed successfully!");
            println!("\n✅ PDF rendering pipeline is working correctly!");
        }
        Err(_) => {
            // Check for fallback
            if session.exp_string("[DEBUG] Failed to display image via Kitty:").is_ok() {
                println!("⚠️  Kitty protocol failed, using fallback");
                println!("\n⚠️  PDF rendering works but Kitty display failed");
            } else {
                println!("\n❌ PDF rendering failed - no display output detected");
            }
        }
    }
    
    // Send escape to exit
    session.send_control('c')?;
    
    // Test with a different PDF if available
    println!("\n--- Testing with Pennsylvania inspection report PDF ---");
    let mut session2 = spawn_bash(Some(10_000))?; // 10 seconds timeout
    session2.send_line("cd /Users/jack/chonker8")?;
    
    // Try to find a PA inspection report PDF
    session2.send_line("ls *.pdf | grep -i 'pa\\|penn\\|inspect' | head -1")?;
    if let Ok(_) = session2.exp_string(".pdf") {
        println!("Found PA inspection PDF, testing...");
        session2.send_line("DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot $(ls *.pdf | grep -i 'pa\\|penn\\|inspect' | head -1)")?;
        
        // Quick check that it loads
        if session2.exp_string("[DEBUG] PDF page rendered").is_ok() {
            println!("✓ PA inspection PDF renders successfully");
        }
    } else {
        println!("No PA inspection PDF found, skipping second test");
    }
    
    session2.send_control('c')?;
    
    println!("\n🎉 All PDF rendering tests completed!");
    
    Ok(())
}