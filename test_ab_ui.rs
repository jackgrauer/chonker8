#!/usr/bin/env rust-script

//! Test the A/B comparison UI with Vello PDF rendering and Kitty display
//! ```cargo
//! [dependencies]
//! chonker8 = { path = "." }
//! anyhow = "1"
//! crossterm = "0.27"
//! image = "0.24"
//! ```

use anyhow::Result;
use chonker8::enhanced_ab_ui::EnhancedABComparison;
use std::path::Path;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::time::Duration;

fn main() -> Result<()> {
    println!("Testing A/B Comparison UI with Vello+Kitty PDF display...\n");
    println!("Loading BERF-CERT.pdf for testing...");
    
    let pdf_path = Path::new("/Users/jack/Desktop/BERF-CERT.pdf");
    
    // Create UI
    let mut ui = EnhancedABComparison::new();
    
    // Load PDF with Vello renderer
    ui.load_pdf_with_vello(pdf_path)?;
    
    // Load some dummy text extraction for the right panel  
    // Note: We don't need to call load_pdf_content since load_pdf_with_vello already set the image
    // Just set the text extraction data directly if needed
    
    // Render the UI
    ui.render_split_view()?;
    
    println!("\nPress 'q' to quit, arrow keys to scroll...");
    
    // Simple event loop
    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('q') => break,
                    KeyCode::Up => {
                        ui.scroll_up();
                        ui.render_split_view()?;
                    }
                    KeyCode::Down => {
                        ui.scroll_down();
                        ui.render_split_view()?;
                    }
                    _ => {}
                }
            }
        }
    }
    
    // Clear Kitty images on exit
    ui.cleanup()?;
    
    println!("\nTest completed!");
    Ok(())
}