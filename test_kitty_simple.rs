#!/usr/bin/env rust-script
//! Test if Kitty graphics protocol works with a simple image

use std::io::{self, Write};

fn main() {
    println!("Testing Kitty Graphics Protocol");
    println!("================================");
    
    // Check environment
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        println!("✅ Kitty detected!");
    } else {
        println!("⚠️  Not running in Kitty terminal");
    }
    
    // Create a simple red square image (PNG format)
    // This is a minimal 2x2 red PNG
    let png_data = vec![
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // PNG signature
        0x00, 0x00, 0x00, 0x0d, // IHDR chunk size
        0x49, 0x48, 0x44, 0x52, // "IHDR"
        0x00, 0x00, 0x00, 0x02, // width: 2
        0x00, 0x00, 0x00, 0x02, // height: 2
        0x08, 0x02, // bit depth: 8, color type: 2 (RGB)
        0x00, 0x00, 0x00, // compression, filter, interlace
        0x72, 0xb6, 0x0d, 0x24, // CRC
        0x00, 0x00, 0x00, 0x0c, // IDAT chunk size
        0x49, 0x44, 0x41, 0x54, // "IDAT"
        0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, // compressed data
        0x18, 0xdd, 0x8d, 0xb4, // CRC
        0x00, 0x00, 0x00, 0x00, // IEND chunk size
        0x49, 0x45, 0x4e, 0x44, // "IEND"
        0xae, 0x42, 0x60, 0x82, // CRC
    ];
    
    // Encode to base64
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    let encoded = BASE64.encode(&png_data);
    
    println!("\nSending test image to Kitty...");
    
    // Clear any existing images
    print!("\x1b_Ga=d,d=a\x1b\\");
    io::stdout().flush().unwrap();
    
    // Send the image with Kitty graphics protocol
    // a=T (transmit), f=100 (PNG), i=1 (image id), s=100 (width), v=100 (height)
    print!("\x1b_Ga=T,f=100,i=1,s=100,v=100;{}\x1b\\", encoded);
    io::stdout().flush().unwrap();
    
    println!("\n✅ Image sent!");
    println!("\nYou should see a red square above if Kitty graphics work.");
    println!("\nNow testing placement at specific position...");
    
    // Move cursor and place image
    print!("\x1b[10;5H"); // Move to row 10, column 5
    print!("\x1b_Ga=p,i=1,x=0,y=0\x1b\\"); // Place image ID 1
    io::stdout().flush().unwrap();
    
    println!("\n\n\n\n\n\n\n\n\nDone! Red square should appear at position (5,10)");
}

// For rust-script
/*
[dependencies]
base64 = "0.22"
*/