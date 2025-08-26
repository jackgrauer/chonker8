#!/usr/bin/env rust-script

//! Test clipboard functionality
//! 
//! ```cargo
//! [dependencies]
//! arboard = "3.3"
//! ```

use arboard::Clipboard;

fn main() {
    println!("Testing clipboard functionality...\n");
    
    // Test 1: Create clipboard
    let mut clipboard = match Clipboard::new() {
        Ok(c) => {
            println!("✓ Clipboard created successfully");
            c
        }
        Err(e) => {
            println!("✗ Failed to create clipboard: {}", e);
            return;
        }
    };
    
    // Test 2: Set text
    let test_text = "Hello from chonker8!";
    match clipboard.set_text(test_text) {
        Ok(_) => println!("✓ Set text: '{}'", test_text),
        Err(e) => println!("✗ Failed to set text: {}", e),
    }
    
    // Test 3: Get text
    match clipboard.get_text() {
        Ok(text) => println!("✓ Got text: '{}'", text),
        Err(e) => println!("✗ Failed to get text: {}", e),
    }
    
    println!("\nClipboard is working!");
}