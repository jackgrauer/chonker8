#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! anyhow = "1.0"
//! ```

use anyhow::Result;
use rexpect::spawn;
use std::time::Duration;
use std::thread;

fn main() -> Result<()> {
    println!("🔍 Testing Visual Split Panel Display");
    println!("{}", "=".repeat(60));
    
    // Test with a simple PDF that should work
    let test_pdf = "/Users/jack/Desktop/BERF-CERT.pdf";
    
    println!("Starting chonker8-hot with PDF...");
    let mut session = spawn("./target/release/chonker8-hot", Some(5000))?;
    
    // Give it time to initialize
    thread::sleep(Duration::from_millis(500));
    
    // Send the PDF path as argument by restarting with it
    session.send_control('c')?; // Kill current session
    thread::sleep(Duration::from_millis(100));
    
    // Start with PDF
    let mut session = spawn(
        &format!("./target/release/chonker8-hot {}", test_pdf),
        Some(10000)
    )?;
    
    println!("Waiting for UI to render...");
    thread::sleep(Duration::from_secs(2));
    
    // Try to capture what's on screen
    println!("Sending Tab to cycle screens...");
    session.send("\t")?;
    thread::sleep(Duration::from_millis(500));
    
    // Try to get some output
    session.send("q")?;
    session.send("\x1b")?; // ESC
    
    println!("\n📋 Debugging the render issue...");
    println!("The UI should show:");
    println!("  - Left panel: PDF rendered image");
    println!("  - Right panel: pdftotext extraction");
    println!("  - Split down the middle");
    
    // Let's check what's actually happening
    println!("\n🔧 Let me check the actual rendering code...");
    
    Ok(())
}