#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! anyhow = "1.0"
//! ```

use anyhow::Result;
use rexpect::spawn_bash;
use std::time::Duration;
use std::thread;

fn main() -> Result<()> {
    println!("🧪 Testing chonker8-hot PDF loading freeze issue...\n");
    
    // First, build the binary
    println!("📦 Building chonker8-hot...");
    let build_output = std::process::Command::new("cargo")
        .args(&["build", "--release", "--bin", "chonker8-hot"])
        .env("DYLD_LIBRARY_PATH", "./lib")
        .current_dir("/Users/jack/chonker8")
        .output()?;
    
    if !build_output.status.success() {
        eprintln!("❌ Build failed!");
        eprintln!("{}", String::from_utf8_lossy(&build_output.stderr));
        return Ok(());
    }
    println!("✅ Build successful\n");
    
    // Test 1: Basic startup
    println!("Test 1: Basic startup without PDF");
    let mut session = spawn_bash(Some(5000))?;
    session.send_line("cd /Users/jack/chonker8")?;
    session.send_line("DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot")?;
    
    // Should see the demo screen
    match session.exp_string("Chonker8.1 Hot-Reload TUI Demo") {
        Ok(_) => {
            println!("✅ Demo screen loaded successfully");
            // Send Tab to switch screens
            session.send("\t")?;
            thread::sleep(Duration::from_millis(500));
            
            // Send Esc to exit
            session.send("\x1b")?;
            thread::sleep(Duration::from_millis(500));
        }
        Err(e) => {
            println!("❌ Failed to load demo screen: {}", e);
            // Try to kill the process
            session.send("\x03")?; // Ctrl+C
        }
    }
    
    println!("\nTest 2: Loading a test PDF");
    
    // Create a simple test PDF first
    create_test_pdf()?;
    
    let mut session2 = spawn_bash(Some(10000))?;
    session2.send_line("cd /Users/jack/chonker8")?;
    session2.send_line("DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot /Users/jack/Documents/test.pdf 2>&1")?;
    
    println!("Waiting for PDF load...");
    
    // Check for the debug messages
    let timeout_ms = 5000;
    let start = std::time::Instant::now();
    let mut got_load_pdf = false;
    let mut got_checking = false;
    let mut got_renderer_call = false;
    let mut got_page_count = false;
    let mut frozen = false;
    
    while start.elapsed().as_millis() < timeout_ms as u128 {
        // Try to read any available output
        match session2.exp_string("[DEBUG] load_pdf called with:") {
            Ok(_) => {
                got_load_pdf = true;
                println!("  ✓ load_pdf called");
            }
            Err(_) => {}
        }
        
        match session2.exp_string("[DEBUG] Checking if path exists: true") {
            Ok(_) => {
                got_checking = true;
                println!("  ✓ Path exists check passed");
            }
            Err(_) => {}
        }
        
        match session2.exp_string("[DEBUG] Path exists, calling renderer.load_pdf") {
            Ok(_) => {
                got_renderer_call = true;
                println!("  ✓ Calling renderer.load_pdf");
            }
            Err(_) => {}
        }
        
        match session2.exp_string("[DEBUG] Getting page count...") {
            Ok(_) => {
                got_page_count = true;
                println!("  ✓ Getting page count...");
                
                // Now wait to see if it completes or freezes
                thread::sleep(Duration::from_secs(2));
                
                match session2.exp_string("[DEBUG] Page count:") {
                    Ok(_) => {
                        println!("  ✅ Page count retrieved successfully!");
                    }
                    Err(_) => {
                        println!("  ⏱️  FROZEN at page count retrieval!");
                        frozen = true;
                    }
                }
                break;
            }
            Err(_) => {}
        }
        
        thread::sleep(Duration::from_millis(100));
    }
    
    // Kill the process
    session2.send("\x03")?; // Ctrl+C
    thread::sleep(Duration::from_millis(500));
    
    println!("\n📊 Test Results:");
    println!("  load_pdf called: {}", if got_load_pdf { "✅" } else { "❌" });
    println!("  Path check: {}", if got_checking { "✅" } else { "❌" });
    println!("  Renderer call: {}", if got_renderer_call { "✅" } else { "❌" });
    println!("  Page count attempt: {}", if got_page_count { "✅" } else { "❌" });
    println!("  Frozen: {}", if frozen { "🔴 YES - This is the bug!" } else { "🟢 NO" });
    
    if frozen {
        println!("\n🔍 Root Cause Analysis:");
        println!("  The freeze occurs when calling content_extractor::get_page_count()");
        println!("  This is likely due to:");
        println!("  1. Multiple Pdfium initialization attempts");
        println!("  2. Library path conflicts between './lib' and './lib/'");
        println!("  3. Thread-local singleton not being used consistently");
        println!("\n💡 Fix: Ensure UIRenderer uses the singleton pattern for all Pdfium calls");
    }
    
    Ok(())
}

fn create_test_pdf() -> Result<()> {
    // Check if test.pdf already exists
    if std::path::Path::new("/Users/jack/Documents/test.pdf").exists() {
        println!("  Using existing test.pdf");
        return Ok(());
    }
    
    println!("  Creating test.pdf...");
    // Create a simple PDF using a Python script
    let python_script = r#"
from reportlab.pdfgen import canvas
c = canvas.Canvas("/Users/jack/Documents/test.pdf")
c.drawString(100, 750, "Test PDF for chonker8")
c.drawString(100, 700, "This is a simple test document")
c.showPage()
c.save()
print("Created test.pdf")
"#;
    
    std::fs::write("/tmp/create_test_pdf.py", python_script)?;
    
    let output = std::process::Command::new("python3")
        .arg("/tmp/create_test_pdf.py")
        .output()?;
    
    if !output.status.success() {
        // If reportlab is not available, create a dummy file
        println!("  reportlab not available, creating dummy PDF");
        std::fs::write("/Users/jack/Documents/test.pdf", "%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\nxref\n0 2\n0000000000 65535 f\n0000000010 00000 n\ntrailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n44\n%%EOF")?;
    }
    
    Ok(())
}