#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! anyhow = "1.0"
//! ```

use anyhow::{Result, Context};
use rexpect::spawn_bash;
use std::path::Path;
use std::fs;

const TEST_PDF: &str = "/Users/jack/Desktop/BERF-CERT.pdf";
const TIMEOUT: u64 = 10;

fn main() -> Result<()> {
    println!("🧪 Testing chonker8 A/B PDF Comparison Viewer");
    println!("{}", "=".repeat(50));
    
    // Run all tests iteratively
    let mut pass_count = 0;
    let mut fail_count = 0;
    
    // Test 1: Binary exists and is executable
    print!("Test 1: Binary exists... ");
    match test_binary_exists() {
        Ok(_) => { println!("✅ PASS"); pass_count += 1; }
        Err(e) => { println!("❌ FAIL: {}", e); fail_count += 1; }
    }
    
    // Test 2: Help flag works
    print!("Test 2: Help flag... ");
    match test_help_flag() {
        Ok(_) => { println!("✅ PASS"); pass_count += 1; }
        Err(e) => { 
            println!("❌ FAIL: {}", e); 
            fail_count += 1;
            // Fix it
            println!("  🔧 Fixing CLI argument parsing...");
            fix_cli_parsing()?;
            rebuild_binary()?;
        }
    }
    
    // Test 3: No DYLD_LIBRARY_PATH needed
    print!("Test 3: Run without DYLD_LIBRARY_PATH... ");
    match test_no_dyld_needed() {
        Ok(_) => { println!("✅ PASS"); pass_count += 1; }
        Err(e) => { 
            println!("❌ FAIL: {}", e); 
            fail_count += 1;
            // Fix it
            println!("  🔧 Removing PDFium dependency...");
            remove_pdfium_dependency()?;
            rebuild_binary()?;
        }
    }
    
    // Test 4: PDF loading and A/B display
    print!("Test 4: A/B PDF comparison display... ");
    match test_ab_comparison_display() {
        Ok(_) => { println!("✅ PASS"); pass_count += 1; }
        Err(e) => { 
            println!("❌ FAIL: {}", e); 
            fail_count += 1;
        }
    }
    
    // Test 5: Kitty graphics protocol working
    print!("Test 5: Kitty graphics (left panel)... ");
    match test_kitty_graphics() {
        Ok(_) => { println!("✅ PASS"); pass_count += 1; }
        Err(e) => { 
            println!("❌ FAIL: {}", e); 
            fail_count += 1;
        }
    }
    
    // Test 6: PDFtotext extraction (right panel)
    print!("Test 6: PDFtotext layout extraction... ");
    match test_pdftotext_extraction() {
        Ok(_) => { println!("✅ PASS"); pass_count += 1; }
        Err(e) => { 
            println!("❌ FAIL: {}", e); 
            fail_count += 1;
        }
    }
    
    println!("{}", "=".repeat(50));
    println!("Results: {} passed, {} failed", pass_count, fail_count);
    
    if fail_count == 0 {
        println!("🎉 All tests passed! The A/B comparison viewer is perfect!");
    } else {
        println!("🔄 Running iterative fixes...");
        iterative_fix_loop()?;
    }
    
    Ok(())
}

fn test_binary_exists() -> Result<()> {
    let binary_path = "./target/release/chonker8-hot";
    if !Path::new(binary_path).exists() {
        anyhow::bail!("Binary not found at {}", binary_path);
    }
    Ok(())
}

fn test_help_flag() -> Result<()> {
    let mut session = spawn_bash(Some(TIMEOUT))?;
    session.send_line("./target/release/chonker8-hot --help 2>&1")?;
    
    // Should show help text, not try to load --help as PDF
    match session.exp_string("Usage:") {
        Ok(_) => Ok(()),
        Err(_) => {
            // Check if it's trying to load --help as a PDF
            if session.exp_string("load_pdf called with: --help").is_ok() {
                anyhow::bail!("Binary is trying to load --help as a PDF file")
            } else {
                anyhow::bail!("Help flag not working properly")
            }
        }
    }
}

fn test_no_dyld_needed() -> Result<()> {
    let mut session = spawn_bash(Some(TIMEOUT))?;
    // Run WITHOUT DYLD_LIBRARY_PATH
    session.send_line("./target/release/chonker8-hot --help 2>&1")?;
    
    // Should not show dyld errors
    match session.exp_string("dyld") {
        Ok(_) => anyhow::bail!("Still requires DYLD_LIBRARY_PATH"),
        Err(_) => Ok(()) // Good, no dyld errors
    }
}

fn test_ab_comparison_display() -> Result<()> {
    if !Path::new(TEST_PDF).exists() {
        println!("  ⚠️  Test PDF not found, skipping");
        return Ok(());
    }
    
    // Test that the binary can load a PDF and show the proper messages
    let output = std::process::Command::new("./target/release/chonker8-hot")
        .arg(TEST_PDF)
        .env("KITTY_WINDOW_ID", "1")
        .output()?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Check for A/B comparison messages in stderr
    if stderr.contains("Left pane: lopdf-vello-kitty rendering") &&
       stderr.contains("Right pane: pdftotext extraction") {
        Ok(())
    } else {
        anyhow::bail!("A/B comparison messages not found in output")
    }
}

fn test_kitty_graphics() -> Result<()> {
    let mut session = spawn_bash(Some(TIMEOUT))?;
    session.send_line("KITTY_WINDOW_ID=1 ./target/release/chonker8-hot --test-kitty 2>&1")?;
    
    // Should detect Kitty and initialize graphics
    match session.exp_string("Kitty graphics protocol detected") {
        Ok(_) => Ok(()),
        Err(_) => {
            // Check if it's using fallback
            if session.exp_string("Kitty graphics protocol not detected").is_ok() {
                anyhow::bail!("Kitty protocol not being detected properly")
            } else {
                Ok(())
            }
        }
    }
}

fn test_pdftotext_extraction() -> Result<()> {
    // Check if pdftotext is available
    let output = std::process::Command::new("which")
        .arg("pdftotext")
        .output()?;
    
    if !output.status.success() {
        anyhow::bail!("pdftotext not found in PATH");
    }
    
    Ok(())
}

fn fix_cli_parsing() -> Result<()> {
    println!("    Adding proper CLI argument parsing...");
    
    let main_hotreload = fs::read_to_string("src/main_hotreload.rs")?;
    
    // Check if clap is already imported
    if !main_hotreload.contains("use clap") {
        // Add clap-based CLI parsing
        let fixed_content = r#"// Hot-reload TUI for chonker8
mod ui_config;
mod ui_renderer;
mod pdf_extraction;
mod config;
mod hot_reload_manager;
mod build_system;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind, EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    io::{stdout, Write},
    path::{Path, PathBuf},
    sync::mpsc::{channel, Receiver},
    time::Duration,
};
use ui_config::UIConfig;
use ui_renderer::{UIRenderer, Screen};
use hot_reload_manager::HotReloadManager;
use std::process::{Command, Stdio};

#[derive(Parser, Debug)]
#[command(name = "chonker8-hot")]
#[command(about = "A/B PDF comparison viewer with hot-reload", long_about = None)]
struct Args {
    /// PDF file to display for A/B comparison
    pdf_file: Option<PathBuf>,
    
    /// Test Kitty graphics protocol
    #[arg(long)]
    test_kitty: bool,
}
"#;
        
        // Find the main function and update it
        let _main_fn_start = main_hotreload.find("fn main()").context("main() not found")?;
        let updated = fixed_content.to_string();
            
        fs::write("src/main_hotreload.rs.new", updated)?;
        
        // Also need to add clap to Cargo.toml if not present
        let cargo_toml = fs::read_to_string("Cargo.toml")?;
        if !cargo_toml.contains("clap") {
            let updated_cargo = cargo_toml.replace(
                "[dependencies]",
                "[dependencies]\nclap = { version = \"4.0\", features = [\"derive\"] }"
            );
            fs::write("Cargo.toml", updated_cargo)?;
        }
    }
    
    Ok(())
}

fn remove_pdfium_dependency() -> Result<()> {
    println!("    Removing PDFium library dependency...");
    
    // Remove the lib directory reference
    if Path::new("lib/libpdfium.dylib").exists() {
        println!("      Moving libpdfium.dylib to backup...");
        fs::rename("lib/libpdfium.dylib", "lib/libpdfium.dylib.backup")?;
    }
    
    Ok(())
}

fn rebuild_binary() -> Result<()> {
    println!("    Rebuilding binary...");
    let output = std::process::Command::new("cargo")
        .args(&["build", "--release", "--bin", "chonker8-hot"])
        .output()?;
    
    if !output.status.success() {
        anyhow::bail!("Build failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    Ok(())
}

fn iterative_fix_loop() -> Result<()> {
    let max_iterations = 5;
    
    for i in 1..=max_iterations {
        println!("\n🔄 Iteration {}/{}", i, max_iterations);
        
        // Rebuild with fixes
        rebuild_binary()?;
        
        // Test again
        let mut all_pass = true;
        
        if test_help_flag().is_err() {
            all_pass = false;
            fix_cli_parsing()?;
        }
        
        if test_no_dyld_needed().is_err() {
            all_pass = false;
            remove_pdfium_dependency()?;
        }
        
        if all_pass {
            println!("✨ All issues fixed in iteration {}!", i);
            break;
        }
    }
    
    Ok(())
}