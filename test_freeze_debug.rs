#!/usr/bin/env rust-script
//! Test script to reproduce and debug the PDF loading freeze
//! 
//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! anyhow = "1.0"
//! ```

use anyhow::Result;
use rexpect::spawn_bash;
use std::time::Duration;
use std::thread;

fn main() -> Result<()> {
    println!("🔍 Testing PDF loading freeze in chonker8-hot...");
    println!("{}", "=".repeat(50));
    
    // Build the binary first
    println!("Building chonker8-hot...");
    std::process::Command::new("bash")
        .arg("-c")
        .arg("DYLD_LIBRARY_PATH=./lib cargo build --release --bin chonker8-hot --quiet")
        .output()?;
    
    println!("✓ Build complete");
    println!();
    
    // Launch chonker8-hot
    println!("Launching chonker8-hot...");
    let mut session = spawn_bash(Some(10000))?;
    session.send_line("DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot")?;
    
    // Wait for initial screen
    thread::sleep(Duration::from_millis(500));
    
    println!("✓ Application launched");
    println!();
    
    // Navigate to file picker (Tab key)
    println!("Navigating to file picker screen...");
    session.send("\t")?;  // Tab to cycle screens
    thread::sleep(Duration::from_millis(500));
    
    println!("✓ On file picker screen");
    println!();
    
    // Try to select a PDF file
    println!("Attempting to load PDF (this is where it freezes)...");
    
    // Type a search query for a PDF
    session.send("BERF")?;
    thread::sleep(Duration::from_millis(500));
    
    // Press Enter to select
    println!("Pressing Enter to load PDF...");
    session.send("\r")?;
    
    // Check if the app responds within 5 seconds
    println!("Waiting for response (5 seconds timeout)...");
    
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(5);
    
    loop {
        if start.elapsed() > timeout {
            println!();
            println!("❌ FREEZE DETECTED!");
            println!("The application is not responding after loading PDF.");
            println!();
            println!("Likely causes:");
            println!("1. Async runtime blocking in load_pdf()");
            println!("2. Nested block_on() calls");
            println!("3. Synchronous operation in async context");
            
            // Kill the frozen process
            session.send("\x03")?; // Ctrl+C
            
            break;
        }
        
        // Try to check if app is responsive
        if let Ok(_) = session.exp_string("PDF loaded") {
            println!("✓ PDF loaded successfully!");
            break;
        }
        
        thread::sleep(Duration::from_millis(100));
    }
    
    println!();
    println!("{}", "=".repeat(50));
    println!("Test complete");
    
    Ok(())
}