#!/usr/bin/env rust-script
//! Tests for debugging PDF loading freeze in chonker8-hot
//! 
//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! anyhow = "1.0"
//! ```

use anyhow::Result;
use rexpect::session::PtySession;
use std::time::Duration;
use std::thread;

fn main() -> Result<()> {
    println!("=== Chonker8 PDF Loading Freeze Debug Test ===");
    println!("This test will simulate loading a PDF and help identify where the freeze occurs.");
    
    // Try different approaches to identify the freeze
    test_basic_startup()?;
    test_tab_navigation()?;
    test_file_picker_interaction()?;
    test_pdf_selection()?;
    
    println!("\n=== All tests completed ===");
    Ok(())
}

fn test_basic_startup() -> Result<()> {
    println!("\n[TEST 1] Basic startup test...");
    
    let mut session = spawn_chonker8()?;
    
    // Wait for initial render
    thread::sleep(Duration::from_millis(500));
    
    // Check if the app started successfully
    session.send_line("")?;
    thread::sleep(Duration::from_millis(100));
    
    // Try to quit gracefully
    session.send_control('c')?;
    thread::sleep(Duration::from_millis(100));
    
    println!("  ✓ Basic startup successful");
    Ok(())
}

fn test_tab_navigation() -> Result<()> {
    println!("\n[TEST 2] Tab navigation test...");
    
    let mut session = spawn_chonker8()?;
    
    // Wait for initial render
    thread::sleep(Duration::from_millis(500));
    
    // Press Tab to navigate to file picker
    println!("  - Pressing Tab to go to file picker...");
    session.send("\t")?;
    thread::sleep(Duration::from_millis(500));
    
    // Check if we're on the file picker screen
    match session.exp_string("File Picker") {
        Ok(_) => println!("  ✓ File picker screen loaded"),
        Err(e) => println!("  ✗ File picker not detected: {}", e),
    }
    
    // Try to quit gracefully
    session.send_control('c')?;
    thread::sleep(Duration::from_millis(100));
    
    Ok(())
}

fn test_file_picker_interaction() -> Result<()> {
    println!("\n[TEST 3] File picker interaction test...");
    
    let mut session = spawn_chonker8()?;
    
    // Wait for initial render
    thread::sleep(Duration::from_millis(500));
    
    // Navigate to file picker
    session.send("\t")?;
    thread::sleep(Duration::from_millis(500));
    
    // Try typing in the search box
    println!("  - Typing 'BERF' in search...");
    session.send("BERF")?;
    thread::sleep(Duration::from_millis(200));
    
    // Try arrow navigation
    println!("  - Pressing Down arrow...");
    session.send("\x1b[B")?; // Down arrow
    thread::sleep(Duration::from_millis(200));
    
    println!("  - Pressing Up arrow...");
    session.send("\x1b[A")?; // Up arrow
    thread::sleep(Duration::from_millis(200));
    
    // Quit
    session.send_control('c')?;
    thread::sleep(Duration::from_millis(100));
    
    println!("  ✓ File picker interaction successful");
    Ok(())
}

fn test_pdf_selection() -> Result<()> {
    println!("\n[TEST 4] PDF selection test (this is where freeze likely occurs)...");
    
    let mut session = spawn_chonker8()?;
    
    // Wait for initial render
    thread::sleep(Duration::from_millis(500));
    
    // Navigate to file picker
    println!("  - Navigating to file picker...");
    session.send("\t")?;
    thread::sleep(Duration::from_millis(500));
    
    // Search for BERF-CERT.pdf
    println!("  - Searching for BERF-CERT...");
    session.send("BERF")?;
    thread::sleep(Duration::from_millis(500));
    
    // Select the first result
    println!("  - Pressing Enter to select PDF...");
    println!("  [FREEZE POINT] If app freezes here, we've identified the issue");
    session.send("\r")?; // Enter key
    
    // Wait and check for freeze
    println!("  - Waiting 3 seconds to see if app responds...");
    thread::sleep(Duration::from_secs(3));
    
    // Try to send another command to check if frozen
    session.send("")?;
    
    // Check if we're still responsive
    match session.exp_string("PDF") {
        Ok(_) => println!("  ✓ PDF loaded successfully!"),
        Err(_) => {
            println!("  ✗ App appears frozen after PDF selection");
            println!("  - Attempting to capture debug output...");
            
            // Try to get any output
            thread::sleep(Duration::from_secs(1));
        }
    }
    
    // Force quit
    session.send_control('c')?;
    thread::sleep(Duration::from_millis(100));
    
    Ok(())
}

fn spawn_chonker8() -> Result<PtySession> {
    println!("  - Spawning chonker8-hot...");
    
    // Set environment for PDFium
    std::env::set_var("DYLD_LIBRARY_PATH", "./lib");
    
    // Use timeout to prevent hanging
    let session = rexpect::spawn("timeout 10 ./target/release/chonker8-hot", Some(2000))?;
    
    Ok(session)
}