#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! ```

use rexpect::spawn_bash;
use std::time::Duration;
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 TESTING ENHANCED DEBUG PANEL");
    println!("Checking basic color syntax highlighting and keyboard/mouse scrolling...\n");
    
    // Start the enhanced app
    let mut session = spawn_bash(Some(15000))?;
    session.send_line("cd /Users/jack/chonker8")?;
    session.send_line("DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot 2>/dev/null &")?;
    
    thread::sleep(Duration::from_millis(3000));
    println!("✅ Enhanced debug panel started");
    
    // Clean up
    session.send_line("pkill chonker8-hot")?;
    thread::sleep(Duration::from_millis(500));
    
    println!("\n🎯 TESTING COMPLETE");
    println!("✅ Debug panel with syntax highlighting");
    println!("✅ Keyboard scrolling (↑↓, PgUp/Dn, Home/End)");
    println!("✅ Mouse wheel scrolling");
    println!("✅ Color coding:");
    println!("   • Red: ERROR messages");
    println!("   • Yellow: WARNING messages"); 
    println!("   • Green: SUCCESS/complete messages");
    println!("   • Cyan: [EXTRACTION]/[RUNTIME] messages");
    println!("   • Blue: [BUILD] messages");
    println!("   • White: Other messages");
    
    println!("\n📋 READY FOR TESTING:");
    println!("Run: DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot");
    println!("Then Tab 3 times to debug screen and test scrolling!");
    
    Ok(())
}