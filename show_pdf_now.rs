#!/usr/bin/env rust-script

//! Direct display of PDF using Vello + Kitty
//! ```cargo
//! [dependencies]
//! chonker8 = { path = "." }
//! anyhow = "1"
//! base64 = "0.21"
//! ```

use anyhow::Result;
use chonker8::vello_pdf_renderer::VelloPdfRenderer;
use std::path::Path;
use std::io::{self, Write};

fn main() -> Result<()> {
    // Render the PDF
    let pdf_path = Path::new("/Users/jack/Desktop/BERF-CERT.pdf");
    let mut renderer = VelloPdfRenderer::new(pdf_path)?;
    let image = renderer.render_page(0, 800, 1000)?;
    
    println!("Rendered PDF: {}x{}", image.width(), image.height());
    
    // Convert to PNG bytes
    let mut png_bytes = Vec::new();
    image.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageOutputFormat::Png)?;
    
    // Send directly via Kitty graphics protocol
    let encoded = base64::encode(&png_bytes);
    
    // Kitty graphics protocol: a=T (transmit), f=100 (PNG format)
    print!("\x1b_Ga=T,f=100,m=1;{}\x1b\\", encoded);
    io::stdout().flush()?;
    
    println!("\n\nPDF displayed above (if terminal supports Kitty graphics)");
    
    Ok(())
}