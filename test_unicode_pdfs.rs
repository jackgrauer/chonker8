#!/usr/bin/env rust-script

//! Test for UTF-8 boundary issues with actual PDFs in Downloads and Desktop
//! 
//! ```cargo
//! [dependencies]
//! anyhow = "1.0"
//! ```

use anyhow::Result;
use std::fs;
use std::path::Path;

fn check_problematic_filenames(dir: &Path) -> Result<Vec<String>> {
    let mut problematic = Vec::new();
    
    if !dir.exists() {
        return Ok(problematic);
    }
    
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            let filename = path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            
            // Check if filename contains multi-byte UTF-8 characters
            if filename.contains('–') || // en-dash
               filename.contains('—') || // em-dash
               filename.contains('\u{2018}') || // smart quote '
               filename.contains('\u{2019}') || // smart quote '
               filename.contains('"') || // smart quote
               filename.contains('"') || // smart quote
               filename.contains('…') || // ellipsis
               filename.contains('€') || // euro
               filename.contains('£') || // pound
               filename.contains('¥') || // yen
               filename.contains('™') || // trademark
               filename.contains('©') || // copyright
               filename.contains('®') || // registered
               filename.contains('°') || // degree
               filename.contains('•') || // bullet
               filename.bytes().any(|b| b > 127) // any non-ASCII
            {
                problematic.push(filename.to_string());
                
                // Test if the old slicing code would crash
                test_string_slicing(filename);
            }
        }
    }
    
    Ok(problematic)
}

fn test_string_slicing(filename: &str) {
    println!("\n  Testing: {}", filename);
    println!("    Length in bytes: {}", filename.len());
    println!("    Length in chars: {}", filename.chars().count());
    
    // Simulate the old problematic code
    let max_width = 112; // The byte index from your error
    
    if filename.len() > max_width {
        // This is what the OLD code would do (and crash)
        println!("    Would truncate at byte {}", max_width);
        
        // Check if byte 112 is a char boundary
        if !filename.is_char_boundary(max_width) {
            println!("    ❌ CRASH! Byte {} is NOT a char boundary!", max_width);
            
            // Find what character spans this byte
            for (i, (byte_idx, ch)) in filename.char_indices().enumerate() {
                if byte_idx <= max_width && byte_idx + ch.len_utf8() > max_width {
                    println!("    ❌ It's inside '{}' (U+{:04X}) at char index {}", 
                             ch, ch as u32, i);
                    println!("    ❌ Character spans bytes {}..{}", 
                             byte_idx, byte_idx + ch.len_utf8());
                    break;
                }
            }
        } else {
            println!("    ✅ Byte {} is a valid char boundary", max_width);
        }
        
        // Show how the FIXED code handles it
        let truncated: String = filename.chars().take(max_width.saturating_sub(3)).collect();
        println!("    Fixed code would show: {}...", truncated);
    }
}

fn main() -> Result<()> {
    println!("Scanning for problematic PDF filenames in Downloads and Desktop");
    println!("{}", "=".repeat(70));
    
    let dirs = [
        ("/Users/jack/Downloads", "Downloads"),
        ("/Users/jack/Desktop", "Desktop"),
    ];
    
    let mut total_problematic = 0;
    
    for (dir_path, dir_name) in dirs {
        println!("\n📂 Checking {}:", dir_name);
        let problematic = check_problematic_filenames(Path::new(dir_path))?;
        
        if problematic.is_empty() {
            println!("  No PDFs with special characters found");
        } else {
            println!("  Found {} PDFs with special characters:", problematic.len());
            total_problematic += problematic.len();
        }
    }
    
    println!("\n{}", "=".repeat(70));
    if total_problematic > 0 {
        println!("⚠️  Found {} PDFs that could cause UTF-8 boundary issues", total_problematic);
        println!("✅ The fix in integrated_file_picker.rs should handle these correctly");
    } else {
        println!("No problematic PDFs found");
    }
    
    Ok(())
}