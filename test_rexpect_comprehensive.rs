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
    println!("🔍 Comprehensive chonker8-hot testing with captured output analysis");
    println!("{}", "=".repeat(80));
    
    // Set Kitty environment for testing
    std::env::set_var("KITTY_WINDOW_ID", "1");
    std::env::set_var("TERM", "xterm-kitty");
    
    // Spawn the app with timeout
    println!("Starting app with PDF...");
    let mut session = spawn(
        "./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf",
        Some(15000)  // 15 second timeout in milliseconds
    )?;
    
    println!("\n📜 Captured Output Analysis:");
    println!("{}", "-".repeat(80));
    
    // Wait for startup
    std::thread::sleep(Duration::from_millis(2000));
    
    // Try to capture startup output
    let mut all_output = String::new();
    let mut analysis_results = Vec::new();
    
    // Read all available output in chunks using available rexpect methods
    for attempt in 0..5 {
        // Try to collect characters into a string
        let mut chunk = String::new();
        let mut chars_read = 0;
        
        // Read characters one by one with timeout
        for _ in 0..1000 { // Max 1000 chars per attempt
            match session.try_read() {
                Some(ch) => {
                    chunk.push(ch);
                    chars_read += 1;
                    
                    // Check for newline to break naturally
                    if ch == '\n' && chars_read > 10 {
                        break;
                    }
                }
                None => break, // No more characters available
            }
        }
        
        if !chunk.is_empty() {
            print!("{}", chunk);
            io::stdout().flush()?;
            all_output.push_str(&chunk);
            
            // Analyze this chunk
            analyze_output_chunk(&chunk, &mut analysis_results);
        } else {
            println!("\n[Attempt {}] No more output available", attempt + 1);
            break;
        }
        
        std::thread::sleep(Duration::from_millis(200));
    }
    
    println!("\n{}", "=".repeat(80));
    
    // Comprehensive analysis
    println!("\n📊 Output Analysis Results:");
    println!("{}", "-".repeat(80));
    
    // Check for key components
    let has_output_capture = all_output.contains("[INFO] Output capture system initialized");
    let has_pdf_loading = all_output.contains("load_pdf called with");
    let has_kitty_protocol = all_output.contains("KITTY");
    let has_simple_kitty = all_output.contains("SIMPLE_KITTY");
    let has_escape_sequences = all_output.contains("\\x1b_G");
    let has_base64_data = all_output.contains("iVBORw0KGgo");  // PNG header in base64
    let has_render_calls = all_output.contains("render() called");
    let has_pdf_screen = all_output.contains("render_pdf_screen");
    let has_vello_rendering = all_output.contains("vello");
    let has_image_dimensions = all_output.contains("Image dimensions");
    
    println!("✓ Output capture system: {}", format_bool(has_output_capture));
    println!("✓ PDF loading process: {}", format_bool(has_pdf_loading));
    println!("✓ Kitty protocol detection: {}", format_bool(has_kitty_protocol));
    println!("✓ Simple Kitty implementation: {}", format_bool(has_simple_kitty));
    println!("✓ Kitty escape sequences: {}", format_bool(has_escape_sequences));
    println!("✓ Base64 image data: {}", format_bool(has_base64_data));
    println!("✓ Render loop execution: {}", format_bool(has_render_calls));
    println!("✓ PDF screen rendering: {}", format_bool(has_pdf_screen));
    println!("✓ Vello GPU rendering: {}", format_bool(has_vello_rendering));
    println!("✓ Image dimension detection: {}", format_bool(has_image_dimensions));
    
    // Advanced analysis
    println!("\n🔬 Advanced Analysis:");
    println!("{}", "-".repeat(80));
    
    // Count debug messages
    let debug_count = all_output.matches("[DEBUG]").count();
    let info_count = all_output.matches("[INFO]").count();
    let warning_count = all_output.matches("[WARNING]").count();
    let error_count = all_output.matches("[ERROR]").count();
    
    println!("Debug messages: {}", debug_count);
    println!("Info messages: {}", info_count);
    println!("Warning messages: {}", warning_count);
    println!("Error messages: {}", error_count);
    
    // Pipeline analysis
    let pipeline_stages = vec![
        ("PDF Loading", all_output.contains("PDF loaded successfully")),
        ("Vello Rendering", all_output.contains("PDF page rendered")),
        ("Kitty Protocol", all_output.contains("Kitty supported")),
        ("Image Transmission", all_output.contains("Sent sized")),
        ("Terminal Setup", all_output.contains("terminal setup")),
        ("Render Loop", all_output.contains("render() complete")),
    ];
    
    println!("\n🔄 Pipeline Stage Analysis:");
    for (stage, passed) in pipeline_stages {
        println!("  {}: {}", stage, format_bool(passed));
    }
    
    // Extract specific data
    println!("\n📏 Extracted Data:");
    println!("{}", "-".repeat(80));
    
    if let Some(dimensions) = extract_image_dimensions(&all_output) {
        println!("Image dimensions: {}", dimensions);
    }
    
    if let Some(pdf_size) = extract_pdf_size(&all_output) {
        println!("PDF size: {}", pdf_size);
    }
    
    // Check for errors or issues
    println!("\n⚠️  Issue Detection:");
    println!("{}", "-".repeat(80));
    
    let issues = vec![
        ("TTY Detection", all_output.contains("Not a TTY")),
        ("Terminal Setup Failed", all_output.contains("Failed to initialize")),
        ("Kitty Not Detected", all_output.contains("No Kitty terminal detected")),
        ("Build Errors", all_output.contains("error:") || all_output.contains("failed")),
        ("Missing Dependencies", all_output.contains("not found")),
    ];
    
    let mut issue_count = 0;
    for (issue, detected) in issues {
        if detected {
            println!("  ❌ {}", issue);
            issue_count += 1;
        } else {
            println!("  ✅ {} (OK)", issue);
        }
    }
    
    // Test interactivity
    println!("\n🎛️  Testing Interactivity:");
    println!("{}", "-".repeat(80));
    
    // Send Tab to switch screens
    println!("Sending Tab key...");
    session.send("\\t")?;
    std::thread::sleep(Duration::from_millis(500));
    
    // Try to read response character by character
    let mut response = String::new();
    for _ in 0..100 { // Max 100 chars
        match session.try_read() {
            Some(ch) => {
                response.push(ch);
                if ch == '\n' && response.len() > 5 {
                    break;
                }
            }
            None => break,
        }
    }
    
    if !response.is_empty() {
        println!("Response to Tab: {} bytes", response.len());
        println!("Tab response data: {}", truncate_string(&response, 200));
    } else {
        println!("No response to Tab key");
    }
    
    // Send Escape to exit
    println!("\n👋 Exiting application...");
    session.send("\\x1b")?;
    std::thread::sleep(Duration::from_millis(500));
    
    // Final summary
    println!("\n📋 Test Summary:");
    println!("{}", "=".repeat(80));
    println!("Total output length: {} characters", all_output.len());
    println!("Debug message count: {}", debug_count);
    println!("Issues detected: {}", issue_count);
    
    let success_score = calculate_success_score(
        has_output_capture,
        has_pdf_loading,
        has_kitty_protocol,
        has_simple_kitty,
        has_render_calls,
        has_pdf_screen,
        issue_count,
    );
    
    println!("Success score: {}%", success_score);
    
    if success_score >= 80 {
        println!("🎉 Test PASSED - Application is working correctly!");
    } else {
        println!("⚠️  Test PARTIAL - Some issues detected");
    }
    
    println!("\n✅ Comprehensive test complete!");
    
    Ok(())
}

fn analyze_output_chunk(data: &str, results: &mut Vec<String>) {
    // Look for key events in this chunk
    if data.contains("Sent sized") {
        results.push("Image transmission detected".to_string());
    }
    if data.contains("render() called") {
        results.push("Render loop active".to_string());
    }
    if data.contains("[ERROR]") {
        results.push("Error detected in output".to_string());
    }
}

fn format_bool(value: bool) -> &'static str {
    if value { "✅ YES" } else { "❌ NO" }
}

fn extract_image_dimensions(output: &str) -> Option<String> {
    // Look for "Image dimensions: WxH"
    if let Some(start) = output.find("Image dimensions: ") {
        let start = start + "Image dimensions: ".len();
        if let Some(end) = output[start..].find('\n') {
            return Some(output[start..start + end].to_string());
        }
    }
    None
}

fn extract_pdf_size(output: &str) -> Option<String> {
    // Look for "PDF: WxH"
    if let Some(start) = output.find("PDF: ") {
        let start = start + "PDF: ".len();
        if let Some(end) = output[start..].find('\n') {
            return Some(output[start..start + end].to_string());
        }
    }
    None
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

fn calculate_success_score(
    has_capture: bool,
    has_pdf_loading: bool,
    has_kitty: bool,
    has_simple_kitty: bool,
    has_render: bool,
    has_pdf_screen: bool,
    issue_count: usize,
) -> u32 {
    let mut score = 0u32;
    
    if has_capture { score += 15; }
    if has_pdf_loading { score += 20; }
    if has_kitty { score += 15; }
    if has_simple_kitty { score += 20; }
    if has_render { score += 15; }
    if has_pdf_screen { score += 15; }
    
    // Subtract points for issues
    let issue_penalty = (issue_count as u32).min(5) * 10;
    score = score.saturating_sub(issue_penalty);
    
    score.min(100)
}