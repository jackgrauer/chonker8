#!/usr/bin/env rust-script

//! Force display the rendered PDF in terminal using Kitty protocol
//! ```cargo
//! [dependencies]
//! image = "0.24"
//! base64 = "0.21"
//! ```

use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Forcing Kitty display of rendered PDF...\n");
    
    // Load the rendered PDF image
    let img = image::open("vello_render_test.png")?;
    println!("Loaded image: {}x{}", img.width(), img.height());
    
    // Convert to PNG bytes
    let mut png_bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)?;
    
    // Base64 encode
    let encoded = base64::encode(&png_bytes);
    println!("Encoded: {} bytes", encoded.len());
    
    // Force Kitty display with different methods
    println!("\nMethod 1: Direct display");
    print!("\x1b_Ga=T,f=100;{}\x1b\\", encoded);
    io::stdout().flush()?;
    
    println!("\n\nMethod 2: With placement");
    print!("\x1b_Ga=T,t=d,f=100;{}\x1b\\", encoded);
    io::stdout().flush()?;
    
    println!("\n\nMethod 3: Scaled display");
    print!("\x1b_Ga=T,f=100,s=400,v=500;{}\x1b\\", encoded);
    io::stdout().flush()?;
    
    println!("\n\nIf you see the birth certificate image above, Kitty is working!");
    println!("Otherwise, try running in a different terminal that supports Kitty graphics.");
    
    Ok(())
}