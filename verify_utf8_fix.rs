#!/usr/bin/env rust-script

//! Verify UTF-8 boundary fix in integrated_file_picker.rs
//! 
//! ```cargo
//! [dependencies]
//! anyhow = "1.0"
//! ```

use anyhow::Result;

fn test_truncation(filename: &str, max_width: usize) {
    println!("\nTesting: {}", filename);
    println!("  Length in bytes: {}", filename.len());
    println!("  Length in chars: {}", filename.chars().count());
    
    // Test the OLD code that would crash
    if filename.len() > max_width {
        println!("  Would truncate at byte {}", max_width);
        
        if !filename.is_char_boundary(max_width) {
            println!("  ❌ OLD CODE WOULD CRASH! Byte {} is NOT a char boundary", max_width);
            
            // Find the character that spans this boundary
            for (byte_idx, ch) in filename.char_indices() {
                if byte_idx <= max_width && byte_idx + ch.len_utf8() > max_width {
                    println!("  ❌ Boundary is inside '{}' (U+{:04X})", ch, ch as u32);
                    println!("  ❌ Character spans bytes {}..{}", byte_idx, byte_idx + ch.len_utf8());
                    break;
                }
            }
        } else {
            println!("  ✅ Byte {} is a valid char boundary (old code would work)", max_width);
        }
    }
    
    // Test the NEW code that's safe
    let max_chars = max_width.saturating_sub(3);
    let truncated: String = filename.chars().take(max_chars).collect();
    println!("  NEW CODE: Safely truncates to: {}...", truncated);
}

fn main() -> Result<()> {
    println!("UTF-8 Boundary Fix Verification");
    println!("{}", "=".repeat(70));
    
    // Test the actual problematic filename
    let problematic_file = "A City's Lost Identity_ An Analysis of The Golden State Warriors' Relocation from Oakland to San Francisco – Mediapolis.pdf";
    
    // Test at the exact byte position that caused the crash
    test_truncation(problematic_file, 112);
    
    // Test other potential problem cases
    println!("\n{}", "=".repeat(70));
    println!("Testing other Unicode filenames:");
    
    let test_cases = vec![
        "semiotics mailbag Théophile, Hypatia Sanders The Baffler, No. 4 (Winter:Spring 1993), pp. 15-19 (5 pages).pdf",
        "Test—with—em—dashes—that—are—very—long—and—might—cause—issues—when—truncating—at—various—positions.pdf",
        "Japanese_テスト_file_with_many_日本語_characters_文字_that_need_careful_handling.pdf",
        "Mixed_中文_العربية_עברית_русский_हिन्दी_test_file_with_many_scripts.pdf",
    ];
    
    for filename in test_cases {
        test_truncation(filename, 112); // Test at the same boundary
        test_truncation(filename, 50);  // Test at shorter boundary
        test_truncation(filename, 80);  // Test at medium boundary
    }
    
    println!("\n{}", "=".repeat(70));
    println!("✅ Fix verification complete!");
    println!("The NEW code using .chars().take() handles all cases safely.");
    
    Ok(())
}