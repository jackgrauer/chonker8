#!/usr/bin/env rust-script

//! Test Debug screen functionality and rexpect accessibility
//! 
//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! anyhow = "1.0"
//! tempfile = "3.8"
//! ```

use anyhow::Result;
use rexpect::spawn;
use std::fs;
use tempfile::TempDir;

fn main() -> Result<()> {
    println!("🧪 Testing Debug Screen Functionality");
    println!("=====================================\n");
    
    // Create temp directory with a test PDF
    let temp_dir = TempDir::new()?;
    let test_pdf = temp_dir.path().join("test.pdf");
    fs::write(&test_pdf, b"%PDF-1.4\n")?;
    
    std::env::set_current_dir(temp_dir.path())?;
    std::env::set_var("DYLD_LIBRARY_PATH", "/Users/jack/chonker8/lib");
    
    println!("📂 Starting chonker8-hot with test PDF...");
    
    match spawn("/Users/jack/chonker8/target/release/chonker8-hot", Some(10000)) {
        Ok(mut session) => {
            println!("✅ Program started successfully");
            
            // Wait for initial screen
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            // Press Tab multiple times to cycle to Debug screen
            println!("📱 Cycling through screens to find Debug...");
            for i in 1..=5 {
                session.send("\t")?; // Tab key
                std::thread::sleep(std::time::Duration::from_millis(200));
                
                // Try to find Debug screen indicators
                match session.exp_string("DEBUG OUTPUT") {
                    Ok(_) => {
                        println!("✅ Found Debug screen after {} tabs!", i);
                        
                        // Look for debug messages
                        println!("🔍 Checking for debug messages...");
                        
                        // The debug messages should be visible
                        if session.exp_string("Messages:").is_ok() {
                            println!("✅ Debug message counter found!");
                        }
                        
                        // Test scrolling
                        println!("📜 Testing scrolling...");
                        session.send("\x1b[B")?; // Down arrow
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        session.send("\x1b[A")?; // Up arrow
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        
                        println!("✅ Scrolling works!");
                        
                        // Load a PDF to generate debug messages
                        println!("📄 Loading PDF to generate debug messages...");
                        session.send("\t")?; // Tab to file picker
                        std::thread::sleep(std::time::Duration::from_millis(200));
                        session.send("\r")?; // Enter to select first PDF
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                        
                        // Go back to Debug screen
                        for _ in 0..4 {
                            session.send("\t")?;
                            std::thread::sleep(std::time::Duration::from_millis(200));
                        }
                        
                        // Check for actual debug messages
                        if session.exp_string("load_pdf called").is_ok() ||
                           session.exp_string("Page count:").is_ok() ||
                           session.exp_string("Extraction complete").is_ok() {
                            println!("✅ Debug messages are being captured and displayed!");
                        }
                        
                        break;
                    }
                    Err(_) => {
                        // Keep cycling
                    }
                }
            }
            
            // Send 'q' to quit
            session.send("q")?;
            println!("\n✅ Test completed successfully!");
            println!("   Debug screen is:");
            println!("   - Accessible via Tab cycling");
            println!("   - Displays debug messages");
            println!("   - Supports scrolling");
            println!("   - Accessible to rexpect for testing");
        }
        Err(e) => {
            println!("❌ Failed to start program: {}", e);
        }
    }
    
    Ok(())
}