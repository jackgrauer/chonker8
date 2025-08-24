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
    println!("🔍 Simple chonker8-hot output capture test");
    println!("{}", "=".repeat(60));
    
    // Set Kitty environment
    std::env::set_var("KITTY_WINDOW_ID", "1");
    std::env::set_var("TERM", "xterm-kitty");
    
    // Spawn the app
    println!("Starting app with timeout...");
    let mut session = spawn(
        "./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf",
        Some(8000)  // 8 second timeout
    )?;
    
    // Wait for startup
    println!("Waiting for startup...");
    std::thread::sleep(Duration::from_millis(2000));
    
    // Collect output
    let mut captured_output = String::new();
    let mut total_chars = 0;
    
    // Read output character by character for 5 seconds
    let start_time = std::time::Instant::now();
    while start_time.elapsed() < Duration::from_secs(5) {
        match session.try_read() {
            Some(ch) => {
                captured_output.push(ch);
                total_chars += 1;
                
                // Print some chars to show progress
                if total_chars % 100 == 0 {
                    print!(".");
                    io::stdout().flush()?;
                }
            }
            None => {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    
    println!("\n\n📊 Analysis Results:");
    println!("{}", "=".repeat(60));
    
    println!("Total characters captured: {}", captured_output.len());
    
    // Key checks
    let has_capture_init = captured_output.contains("Output capture system initialized");
    let has_pdf_loading = captured_output.contains("load_pdf called with");
    let has_kitty_protocol = captured_output.contains("KITTY");
    let has_simple_kitty = captured_output.contains("SIMPLE_KITTY");
    let has_render_calls = captured_output.contains("render() called");
    let has_escape_sequences = captured_output.contains("\\x1b_G") || captured_output.contains("_Ga=");
    let has_base64_data = captured_output.contains("iVBORw0KGgo");
    
    println!("✓ Capture system initialized: {}", format_result(has_capture_init));
    println!("✓ PDF loading detected: {}", format_result(has_pdf_loading));
    println!("✓ Kitty protocol active: {}", format_result(has_kitty_protocol));
    println!("✓ Simple Kitty used: {}", format_result(has_simple_kitty));
    println!("✓ Render loop active: {}", format_result(has_render_calls));
    println!("✓ Kitty escape sequences: {}", format_result(has_escape_sequences));
    println!("✓ Base64 image data: {}", format_result(has_base64_data));
    
    // Count debug messages
    let debug_count = captured_output.matches("[DEBUG]").count();
    let info_count = captured_output.matches("[INFO]").count();
    let warning_count = captured_output.matches("[WARNING]").count();
    
    println!("\n📈 Message counts:");
    println!("  Debug: {}", debug_count);
    println!("  Info: {}", info_count);
    println!("  Warnings: {}", warning_count);
    
    // Show a sample of output (first 500 chars)
    println!("\n📋 Sample Output (first 500 chars):");
    println!("{}", "-".repeat(60));
    let sample = if captured_output.len() > 500 {
        format!("{}...", &captured_output[..500])
    } else {
        captured_output.clone()
    };
    println!("{}", sample);
    
    // Calculate success score
    let mut score = 0;
    if has_capture_init { score += 15; }
    if has_pdf_loading { score += 20; }
    if has_kitty_protocol { score += 15; }
    if has_simple_kitty { score += 20; }
    if has_render_calls { score += 15; }
    if has_escape_sequences { score += 10; }
    if has_base64_data { score += 5; }
    
    println!("\n🎯 Final Score: {}%", score);
    
    if score >= 80 {
        println!("🎉 SUCCESS: Hot reload output capture is working!");
    } else if score >= 60 {
        println!("⚠️  PARTIAL: Some components working");
    } else {
        println!("❌ FAILURE: Major issues detected");
    }
    
    // Try to gracefully exit
    println!("\n👋 Sending exit signal...");
    match session.send("\\x1b") {
        Ok(_) => println!("Exit signal sent"),
        Err(_) => println!("Could not send exit signal"),
    }
    
    println!("\n✅ Test complete!");
    
    Ok(())
}

fn format_result(success: bool) -> &'static str {
    if success { "✅ YES" } else { "❌ NO" }
}