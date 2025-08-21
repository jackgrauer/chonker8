#!/usr/bin/env rust-script

//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! anyhow = "1.0"
//! ```

use anyhow::Result;
use rexpect::spawn_bash;
use std::time::Duration;

fn main() -> Result<()> {
    println!("🧪 Testing integrated file picker with rexpect...");

    // Start the chonker8-hot application
    let mut session = spawn_bash(Some(Duration::from_millis(2000)))?;
    
    // Set the library path and run the app
    session.send_line("export DYLD_LIBRARY_PATH=./lib")?;
    session.send_line("./target/release/chonker8-hot")?;
    
    // Wait for the app to initialize
    session.exp_regex(r"Starting in demo mode...")?;
    println!("✅ App started in demo mode");
    
    // Press Tab to go to file picker screen
    session.send_control('i')?; // Tab key
    std::thread::sleep(Duration::from_millis(500));
    
    // Capture the output to see what's rendered
    let output = session.read_string()?;
    println!("📋 Current screen output:");
    println!("{}", output);
    
    // Check if we see the integrated file picker or the fallback
    if output.contains("🎯 Calling integrated file picker render method") {
        println!("✅ SUCCESS: Integrated file picker is rendering!");
    } else if output.contains("⚠️ Using FALLBACK file picker screen") {
        println!("❌ PROBLEM: Still using fallback file picker");
    } else if output.contains("Search: (type to search files)") {
        println!("🔍 File picker screen is showing, checking which version...");
        
        // Try typing to see if search works
        session.send_line("test")?;
        std::thread::sleep(Duration::from_millis(300));
        
        let search_output = session.read_string()?;
        println!("🔤 Search test output:");
        println!("{}", search_output);
    }
    
    // Test Tab cycling through all screens
    println!("🔄 Testing screen cycling...");
    for i in 0..5 {
        session.send_control('i')?; // Tab
        std::thread::sleep(Duration::from_millis(300));
        let screen_output = session.read_string()?;
        println!("📺 Screen {} output preview: {}", i+2, 
                screen_output.lines().next().unwrap_or("").trim());
    }
    
    // Exit gracefully
    session.send_control('c')?; // Ctrl+C to exit
    
    println!("🏁 Test completed!");
    Ok(())
}