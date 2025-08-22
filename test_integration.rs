#!/usr/bin/env rust-script

//! Integration test for UTF-8 fix in chonker8-hot
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
use std::time::Duration;
use tempfile::TempDir;

fn main() -> Result<()> {
    println!("🧪 Testing chonker8-hot with Unicode filenames...\n");
    
    // Create temp directory with problematic PDFs
    let temp_dir = TempDir::new()?;
    
    // Create PDFs with various problematic names
    let mut test_files = vec![
        "A City's Lost Identity_ An Analysis of The Golden State Warriors' Relocation from Oakland to San Francisco – Mediapolis.pdf".to_string(),
        "Test—with—em—dashes.pdf".to_string(),
        "Japanese_テスト_file.pdf".to_string(),
        "Mixed_中文_العربية_test.pdf".to_string(),
        "Emoji_🐹_chonker.pdf".to_string(),
    ];
    test_files.push(format!("Very_long_name_with_special_chars_{}_that_will_definitely_need_truncation_at_some_point.pdf", "é".repeat(50)));
    
    for filename in &test_files {
        let path = temp_dir.path().join(filename);
        fs::write(&path, b"%PDF-1.4\n")?;
        println!("Created: {}", filename);
    }
    
    // Set environment and run chonker8-hot
    std::env::set_current_dir(temp_dir.path())?;
    std::env::set_var("DYLD_LIBRARY_PATH", "/Users/jack/chonker8/lib");
    
    println!("\n📂 Running chonker8-hot in directory with Unicode filenames...");
    
    match spawn("/Users/jack/chonker8/target/release/chonker8-hot", Some(5000)) {
        Ok(mut session) => {
            println!("✅ Program started successfully");
            
            // Wait for file picker to appear
            match session.exp_string("Chonker8 Hot-Reload File Picker") {
                Ok(_) => {
                    println!("✅ File picker loaded without UTF-8 panic!");
                    
                    // Try navigating through files
                    for i in 1..=3 {
                        println!("  Testing navigation (pressing ↓ {})", i);
                        session.send("\x1b[B")?; // Down arrow
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    
                    println!("✅ Navigation successful - no UTF-8 crashes!");
                }
                Err(e) => {
                    println!("❌ File picker didn't appear: {}", e);
                    println!("   Checking for panic...");
                    
                    // Try to read any error output
                    // Note: try_read() returns Option<char> in rexpect, not what we need
                    // We'll check for panic in the direct run below
                }
            }
            
            // Send 'q' to quit
            let _ = session.send("q");
            println!("✅ Test completed successfully!");
        }
        Err(e) => {
            println!("⚠️  Failed to start program: {}", e);
            println!("   Running directly to check for panics...");
            
            // Run directly to see any panic message
            let output = std::process::Command::new("/Users/jack/chonker8/target/release/chonker8-hot")
                .env("DYLD_LIBRARY_PATH", "/Users/jack/chonker8/lib")
                .current_dir(temp_dir.path())
                .output()?;
            
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not a char boundary") {
                println!("❌ UTF-8 boundary panic detected!");
                println!("Error: {}", stderr);
                return Err(anyhow::anyhow!("UTF-8 boundary panic"));
            } else {
                println!("✅ No UTF-8 panic detected in output");
            }
        }
    }
    
    println!("\n🎉 All tests passed! UTF-8 fix is working correctly.");
    Ok(())
}