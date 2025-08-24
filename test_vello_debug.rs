#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! chonker8 = { path = "." }
//! anyhow = "1.0"
//! image = "0.25"
//! ```

use anyhow::Result;
use chonker8::pdf_renderer;
use std::path::Path;

fn main() -> Result<()> {
    println!("🔍 Debug Vello PDF Renderer");
    println!("{}", "=".repeat(60));
    
    // Add explicit debugging
    println!("Starting debug process...");
    
    let pdf_path = Path::new("/Users/jack/Desktop/BERF-CERT.pdf");
    
    if !pdf_path.exists() {
        println!("❌ PDF not found: {:?}", pdf_path);
        return Ok(());
    }
    
    println!("✅ PDF found: {:?}", pdf_path);
    println!("📊 Testing different sizes...");
    
    // Test small size first
    println!("\n🎯 Rendering 400x600...");
    match pdf_renderer::render_pdf_page(pdf_path, 0, 400, 600) {
        Ok(image) => {
            let (width, height) = (image.width(), image.height());
            println!("✅ Success! Image: {}x{}", width, height);
            
            // Save and analyze the image
            image.save("debug_vello_400x600.png")?;
            println!("💾 Saved: debug_vello_400x600.png");
            
            // Check if the image has actual content by looking at pixel values
            let rgba = image.to_rgba8();
            let mut white_pixels = 0u32;
            let mut black_pixels = 0u32;
            let mut other_pixels = 0u32;
            
            // Sample pixels to check content
            let total_pixels = (width * height) as u32;
            let sample_size = (total_pixels / 100).max(1000).min(total_pixels); // Sample 1% or 1000 pixels
            
            for i in (0..total_pixels).step_by((total_pixels / sample_size) as usize) {
                let x = i % width;
                let y = i / width;
                if y < height {
                    let pixel = rgba.get_pixel(x, y);
                    let [r, g, b, _a] = pixel.0;
                    
                    if r > 240 && g > 240 && b > 240 {
                        white_pixels += 1;
                    } else if r < 50 && g < 50 && b < 50 {
                        black_pixels += 1;
                    } else {
                        other_pixels += 1;
                    }
                }
            }
            
            println!("\n📊 Pixel Analysis (sampled {} pixels):", sample_size);
            println!("   White pixels: {} ({:.1}%)", white_pixels, (white_pixels as f32 / sample_size as f32) * 100.0);
            println!("   Black pixels: {} ({:.1}%)", black_pixels, (black_pixels as f32 / sample_size as f32) * 100.0);
            println!("   Other pixels: {} ({:.1}%)", other_pixels, (other_pixels as f32 / sample_size as f32) * 100.0);
            
            if white_pixels > (sample_size * 90 / 100) {
                println!("⚠️  WARNING: Image is mostly white - likely empty content!");
            } else if black_pixels > 0 || other_pixels > 0 {
                println!("✅ Good: Image contains visible content");
            }
        }
        Err(e) => {
            println!("❌ Failed: {}", e);
        }
    }
    
    // Test standard UI size
    println!("\n🎯 Rendering 2400x3200 (UI size)...");
    match pdf_renderer::render_pdf_page(pdf_path, 0, 2400, 3200) {
        Ok(image) => {
            let (width, height) = (image.width(), image.height());
            println!("✅ Success! Image: {}x{}", width, height);
            
            image.save("debug_vello_2400x3200.png")?;
            println!("💾 Saved: debug_vello_2400x3200.png");
            
            // Quick check - just check a few pixels from different areas
            let rgba = image.to_rgba8();
            let center_pixel = rgba.get_pixel(width/2, height/2);
            let corner_pixel = rgba.get_pixel(width/4, height/4);
            let bottom_pixel = rgba.get_pixel(width/2, height*3/4);
            
            println!("🔍 Sample pixels:");
            println!("   Center: {:?}", center_pixel.0);
            println!("   Corner: {:?}", corner_pixel.0);
            println!("   Bottom: {:?}", bottom_pixel.0);
        }
        Err(e) => {
            println!("❌ Failed: {}", e);
        }
    }
    
    println!("\n🎯 Next Steps:");
    println!("1. Check the saved PNG files in an image viewer");
    println!("2. If blank/white, the Vello renderer needs content parsing fixes");
    println!("3. If content is visible, the Kitty protocol transmission is the issue");
    
    Ok(())
}