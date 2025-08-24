use image::DynamicImage;
use anyhow::Result;
use std::io::{self, Write};

/// Minimal Kitty graphics implementation that actually works
pub struct SimpleKitty;

impl SimpleKitty {
    /// Send image using the simplest possible Kitty protocol
    pub fn send_image(image: &DynamicImage) -> Result<()> {
        // Convert to PNG
        let mut png_data = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)?;
        
        // Base64 encode the entire image
        use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
        let encoded = BASE64.encode(&png_data);
        
        // Clear any existing images
        print!("\x1b_Ga=d\x1b\\");
        io::stdout().flush()?;
        
        // Send the entire image in one go - no chunking
        // a=T (transmit), f=100 (PNG)
        print!("\x1b_Ga=T,f=100;{}\x1b\\", encoded);
        io::stdout().flush()?;
        
        eprintln!("[SIMPLE_KITTY] Sent {} bytes as {} base64 chars", png_data.len(), encoded.len());
        
        Ok(())
    }
    
    /// Send image with explicit dimensions and place it immediately
    pub fn send_image_sized(image: &DynamicImage, width: u32, height: u32) -> Result<()> {
        // Convert to PNG
        let mut png_data = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)?;
        
        // Base64 encode
        use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
        let encoded = BASE64.encode(&png_data);
        
        // Clear existing
        print!("\x1b_Ga=d\x1b\\");
        io::stdout().flush()?;
        
        // Transmit and place in one go
        // a=T (transmit), f=100 (PNG), s=width, v=height, U=1 (place after transmission)
        print!("\x1b_Ga=T,f=100,s={},v={},U=1;{}\x1b\\", width, height, encoded);
        io::stdout().flush()?;
        
        eprintln!("[SIMPLE_KITTY] Transmitted and placed {}x{}", width, height);
        
        Ok(())
    }
    
    /// Send image with positioning
    pub fn send_image_positioned(image: &DynamicImage, width: u32, height: u32, x: u16, y: u16) -> Result<()> {
        // Convert to PNG
        let mut png_data = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)?;
        
        // Base64 encode
        use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
        let encoded = BASE64.encode(&png_data);
        
        // Clear existing
        print!("\x1b_Ga=d\x1b\\");
        io::stdout().flush()?;
        
        // First transmit the image with an ID
        print!("\x1b_Ga=T,f=100,s={},v={},i=1;{}\x1b\\", width, height, encoded);
        io::stdout().flush()?;
        
        // Then place it at specific coordinates
        // a=p (place), i=1 (image ID), X=column, Y=row  
        print!("\x1b_Ga=p,i=1,X={},Y={}\x1b\\", x, y);
        io::stdout().flush()?;
        
        eprintln!("[SIMPLE_KITTY] Positioned at ({},{}) size {}x{}", x, y, width, height);
        
        Ok(())
    }
}