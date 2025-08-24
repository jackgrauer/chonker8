#!/usr/bin/env rust-script

//! Test the complete lopdf-vello-kitty pipeline
//! ```cargo
//! [dependencies]
//! chonker8 = { path = "." }
//! anyhow = "1"
//! ```

use anyhow::Result;
use chonker8::vello_pdf_renderer::VelloPdfRenderer;
use chonker8::kitty_protocol::KittyProtocol;
use std::path::Path;

fn main() -> Result<()> {
    println!("Testing complete lopdf→vello→kitty pipeline...");
    
    let pdf_path = Path::new("/Users/jack/Desktop/BERF-CERT.pdf");
    
    // Step 1: lopdf→vello rendering
    println!("Step 1: Rendering PDF with Vello...");
    let mut renderer = VelloPdfRenderer::new(pdf_path)?;
    let image = renderer.render_page(0, 800, 1000)?;
    println!("✓ Rendered {}x{} image", image.width(), image.height());
    
    // Step 2: vello→kitty display
    println!("Step 2: Displaying with Kitty protocol...");
    let mut kitty = KittyProtocol::new();
    kitty.display_image(&image, 0, 0, None, None)?;
    println!("✓ Sent image to terminal");
    
    println!("\n✅ Pipeline test successful!");
    println!("The PDF should be visible above if your terminal supports Kitty graphics.");
    
    Ok(())
}