#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! anyhow = "1.0"
//! ```

use anyhow::Result;
use rexpect::spawn;
use std::time::Duration;
use std::io::{self, Write};

fn main() -> Result<()> {
    println!("🔍 Testing chonker8-hot with full feedback capture");
    println!("{}", "=".repeat(60));
    
    // Set Kitty environment
    std::env::set_var("KITTY_WINDOW_ID", "1");
    std::env::set_var("TERM", "xterm-kitty");
    
    // Spawn the app
    println!("Starting app...");
    let mut session = spawn(
        "./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf",
        Some(10000)  // 10 second timeout in milliseconds
    )?;
    
    // Wait a moment for startup
    std::thread::sleep(Duration::from_millis(1000));
    
    // Try to read all available output
    println!("\n📜 Captured Output:");
    println!("{}", "-".repeat(60));
    
    let mut full_output = String::new();
    loop {
        // Try to read any available data
        match session.exp_any(vec![rexpect::EOF]) {
            Ok((data, _)) => {
                print!("{}", data);
                io::stdout().flush()?;
                full_output.push_str(&data);
                break;  // Got all data up to EOF or timeout
            }
            Err(_) => {
                // Try to get partial data
                match session.exp_timeout(100) {
                    Ok(data) => {
                        print!("{}", data);
                        io::stdout().flush()?;
                        full_output.push_str(&data);
                    }
                    Err(_) => break,  // No more data
                }
            }
        }
    }
    
    println!("\n{}", "-".repeat(60));
    
    // Analyze what we captured
    println!("\n📊 Analysis:");
    println!("{}", "-".repeat(60));
    
    // Check for Kitty escape sequences
    let kitty_clear = full_output.contains("\x1b_Ga=d");
    let kitty_transmit = full_output.contains("\x1b_Ga=T");
    let has_png = full_output.contains("f=100");
    
    println!("✓ Kitty clear command present: {}", kitty_clear);
    println!("✓ Kitty transmit command present: {}", kitty_transmit);
    println!("✓ PNG format specified: {}", has_png);
    
    // Check for debug messages
    let has_debug = full_output.contains("[DEBUG]");
    let has_kitty_debug = full_output.contains("[KITTY]");
    let has_simple = full_output.contains("SIMPLE_KITTY");
    
    println!("✓ Debug messages: {}", has_debug);
    println!("✓ Kitty protocol messages: {}", has_kitty_debug);
    println!("✓ Simple Kitty messages: {}", has_simple);
    
    // Show hex dump of escape sequences
    println!("\n🔬 Escape Sequence Analysis:");
    println!("{}", "-".repeat(60));
    
    // Find and display Kitty escape sequences
    let bytes = full_output.as_bytes();
    for i in 0..bytes.len() {
        if i + 3 < bytes.len() && &bytes[i..i+3] == b"\x1b_G" {
            // Found a Kitty graphics sequence
            print!("\nFound Kitty sequence at position {}: ", i);
            
            // Print up to 50 chars or until ESC\
            let mut j = i;
            while j < bytes.len().min(i + 50) {
                if j + 2 < bytes.len() && &bytes[j..j+2] == b"\x1b\\" {
                    print!("ESC\\");
                    break;
                }
                if bytes[j].is_ascii_graphic() || bytes[j] == b' ' {
                    print!("{}", bytes[j] as char);
                } else if bytes[j] == 0x1b {
                    print!("ESC");
                } else {
                    print!("[{:02x}]", bytes[j]);
                }
                j += 1;
            }
            println!();
        }
    }
    
    // Try sending Tab to switch screens
    println!("\n🔄 Sending Tab key to switch screens...");
    session.send("\t")?;
    std::thread::sleep(Duration::from_millis(500));
    
    // Read new output
    match session.exp_timeout(500) {
        Ok(data) => {
            println!("After Tab:");
            println!("{}", data);
        }
        Err(_) => {
            println!("No new output after Tab");
        }
    }
    
    // Send Escape to exit
    println!("\n👋 Sending Escape to exit...");
    session.send("\x1b")?;
    
    println!("\n✅ Test complete!");
    
    Ok(())
}