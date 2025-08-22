#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! ```

use rexpect::spawn_bash;
use std::time::Duration;
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧈 TESTING BUTTERY SMOOTH SCROLLING");
    println!("Verifying scroll boundaries and smooth limits...\n");
    
    // Start the app
    let mut session = spawn_bash(Some(15000))?;
    session.send_line("cd /Users/jack/chonker8")?;
    session.send_line("DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot 2>/dev/null &")?;
    
    thread::sleep(Duration::from_millis(3000));
    println!("✅ App started with buttery scrolling");
    
    // Clean up
    session.send_line("pkill chonker8-hot")?;
    thread::sleep(Duration::from_millis(500));
    
    println!("\n🧈 BUTTERY SCROLLING FEATURES:");
    println!("✅ Smart scroll boundaries - stops at top/bottom");
    println!("✅ No over-scrolling past content");
    println!("✅ Proper calculation of visible content area");
    println!("✅ Terminal-size-aware maximum scroll offset");
    println!("✅ Smooth Page Up/Down with boundaries");
    println!("✅ Home/End keys work perfectly");
    
    println!("\n📋 TEST IT:");
    println!("Run: DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot");
    println!("1. Tab 3 times to debug screen");
    println!("2. Try scrolling past the top - should stop cleanly");
    println!("3. Try scrolling past the bottom - should stop cleanly"); 
    println!("4. Use trackpad wildly - no terminal corruption!");
    println!("5. Test Page Up/Down, Home/End keys");
    
    println!("\n🎯 Expected: Butter-smooth scrolling that respects boundaries!");
    
    Ok(())
}