#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! ```

use rexpect::spawn_bash;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Kitty Protocol in chonker8-hot");
    println!("=======================================");
    
    // Create a simple test PDF
    println!("Creating test PDF...");
    std::process::Command::new("bash")
        .arg("-c")
        .arg("echo 'Test PDF Content' | ps2pdf - /tmp/test_kitty.pdf 2>/dev/null")
        .output()?;
    
    // Test 1: Check if Kitty is detected
    println!("\nTest 1: Kitty Detection");
    println!("-----------------------");
    
    let mut session = spawn_bash(Some(5000))?;
    
    // Set KITTY_WINDOW_ID to simulate Kitty terminal
    session.send_line("export KITTY_WINDOW_ID=1")?;
    session.send_line("cd /Users/jack/chonker8")?;
    
    // Run chonker8-hot with debug output
    session.send_line("DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot /tmp/test_kitty.pdf 2>&1 &")?;
    session.send_line("CHONKER_PID=$!")?;
    
    // Wait a bit for startup
    std::thread::sleep(Duration::from_millis(500));
    
    // Check for Kitty detection
    session.send_line("sleep 0.5")?;
    
    // Look for debug output
    match session.exp_string("[DEBUG] Kitty graphics protocol detected") {
        Ok(_) => println!("✓ Kitty protocol detected successfully"),
        Err(e) => {
            println!("✗ Kitty protocol not detected: {}", e);
            
            // Try to get more debug info
            println!("\nDebug: Checking environment...");
            session.send_line("echo $KITTY_WINDOW_ID")?;
            if let Ok(output) = session.exp_string("1") {
                println!("  KITTY_WINDOW_ID is set correctly");
            }
        }
    }
    
    // Test 2: Check if image is being transmitted
    println!("\nTest 2: Image Transmission");
    println!("---------------------------");
    
    // Look for image transmission sequences
    session.send_line("sleep 0.5")?;
    
    // Check for debug messages about image display
    let checks = [
        ("PDF image size:", "PDF dimensions detected"),
        ("Display size:", "Image scaling calculated"),
        ("Successfully displayed image", "Image transmitted via Kitty"),
        ("\\x1b_G", "Kitty escape sequence sent"),
    ];
    
    for (pattern, description) in &checks {
        match session.exp_string(pattern) {
            Ok(_) => println!("✓ {}", description),
            Err(_) => println!("✗ {} not found", description),
        }
    }
    
    // Test 3: Interactive commands
    println!("\nTest 3: Interactive Navigation");
    println!("-------------------------------");
    
    // Try navigation commands
    session.send_line("q")?;  // Quit command
    
    // Check if it exits cleanly
    session.send_line("wait $CHONKER_PID 2>/dev/null")?;
    session.send_line("echo \"Exit code: $?\"")?;
    
    match session.exp_string("Exit code: 0") {
        Ok(_) => println!("✓ Clean exit"),
        Err(_) => {
            println!("✗ Did not exit cleanly");
            // Force kill if needed
            session.send_line("kill $CHONKER_PID 2>/dev/null")?;
        }
    }
    
    // Test 4: Test without Kitty (fallback mode)
    println!("\nTest 4: Fallback Mode (no Kitty)");
    println!("---------------------------------");
    
    session.send_line("unset KITTY_WINDOW_ID")?;
    session.send_line("DYLD_LIBRARY_PATH=./lib timeout 1 ./target/release/chonker8-hot /tmp/test_kitty.pdf 2>&1 | head -20 &")?;
    
    std::thread::sleep(Duration::from_millis(500));
    
    // Check for fallback mode
    match session.exp_string("Kitty graphics protocol not detected") {
        Ok(_) => println!("✓ Fallback mode activated"),
        Err(_) => println!("✗ Fallback detection unclear"),
    }
    
    println!("\n=======================================");
    println!("Test Complete!");
    
    Ok(())
}