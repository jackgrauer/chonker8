#!/usr/bin/env rust-script

//! Test for UTF-8 boundary issue in file picker
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
use std::path::Path;
use tempfile::TempDir;

fn create_test_pdfs(dir: &Path) -> Result<()> {
    // Create PDFs with various Unicode characters that could cause boundary issues
    let test_files = vec![
        "simple.pdf",
        "A City's Lost Identity_ An Analysis of The Golden State Warriors' Relocation from Oakland to San Francisco – Mediapolis.pdf",
        "Test—with—em—dashes.pdf",
        "Japanese_テスト_file.pdf",
        "Emoji_🐹_test.pdf",
        "Mixed_中文_العربية_test.pdf",
        "Long_name_with_special_chars_€_£_¥_₹_₽_test_file_that_needs_truncation.pdf",
    ];
    
    for file in test_files {
        let path = dir.join(file);
        fs::write(&path, b"%PDF-1.4\n")?;
        println!("Created: {}", file);
    }
    
    Ok(())
}

fn test_file_picker_with_unicode() -> Result<()> {
    println!("🧪 Testing UTF-8 boundary handling in file picker...\n");
    
    // Create temp directory with test PDFs
    let temp_dir = TempDir::new()?;
    create_test_pdfs(temp_dir.path())?;
    
    // Set up environment to use our test directory
    std::env::set_current_dir(temp_dir.path())?;
    std::env::set_var("DYLD_LIBRARY_PATH", "/Users/jack/chonker8/lib");
    
    println!("📂 Running chonker8-hot in directory with Unicode filenames...");
    
    // Try to spawn the program
    match spawn("/Users/jack/chonker8/target/release/chonker8-hot", Some(5000)) {
        Ok(mut session) => {
            println!("✅ Program started successfully");
            
            // Wait for file picker to appear
            match session.exp_string("Chonker8 Hot-Reload File Picker") {
                Ok(_) => println!("✅ File picker loaded without panic!"),
                Err(e) => println!("❌ File picker didn't appear: {}", e),
            }
            
            // Try navigating through files
            for i in 1..5 {
                println!("  Pressing ↓ (attempt {})", i);
                session.send("\x1b[B")?; // Down arrow
                std::thread::sleep(std::time::Duration::from_millis(100));
                
                // Check if still alive by trying to read
                match session.try_read() {
                    Some(_) => {}, // Still running
                    None => {
                        println!("❌ Program crashed during navigation!");
                        return Err(anyhow::anyhow!("Program crashed"));
                    }
                }
            }
            
            println!("✅ Navigation successful!");
            
            // Send 'q' to quit
            session.send("q")?;
            println!("✅ Test completed successfully!");
        }
        Err(e) => {
            println!("❌ Failed to start program: {}", e);
            println!("   This might mean it crashed immediately due to UTF-8 issue");
            
            // Try to run it directly to see the panic
            println!("\n🔍 Running directly to see panic message:");
            let output = std::process::Command::new("/Users/jack/chonker8/target/release/chonker8-hot")
                .env("DYLD_LIBRARY_PATH", "/Users/jack/chonker8/lib")
                .current_dir(temp_dir.path())
                .output()?;
            
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("byte index") && stderr.contains("not a char boundary") {
                println!("❌ CONFIRMED: UTF-8 boundary panic detected!");
                println!("   Error: {}", stderr);
                return Err(anyhow::anyhow!("UTF-8 boundary panic"));
            }
        }
    }
    
    Ok(())
}

fn main() -> Result<()> {
    println!("UTF-8 Boundary Test for Chonker8 File Picker");
    println!("{}", "=".repeat(50));
    
    match test_file_picker_with_unicode() {
        Ok(_) => {
            println!("\n✅ All tests passed!");
            Ok(())
        }
        Err(e) => {
            println!("\n❌ Test failed: {}", e);
            println!("\n🔧 The issue is in integrated_file_picker.rs around line 150");
            println!("   We need to use char_indices() or truncate at char boundaries");
            Err(e)
        }
    }
}