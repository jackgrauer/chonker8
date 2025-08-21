use anyhow::Result;

// Import from the crate's module structure
#[path = "../pdf_extraction/trocr_extraction.rs"]
mod trocr_extraction;
use trocr_extraction::SimpleTrOCR;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing TrOCR Integration\n");
    
    // Create a simple test image (white background with black text)
    let width = 384;
    let height = 384;
    let mut img = image::RgbImage::new(width, height);
    
    // Fill with white
    for pixel in img.pixels_mut() {
        *pixel = image::Rgb([255, 255, 255]);
    }
    
    // Add some black pixels to simulate text
    for x in 50..100 {
        for y in 50..60 {
            img.put_pixel(x, y, image::Rgb([0, 0, 0]));
        }
    }
    
    // Convert to bytes
    let mut buffer = Vec::new();
    image::DynamicImage::ImageRgb8(img).write_to(
        &mut std::io::Cursor::new(&mut buffer),
        image::ImageFormat::Png
    )?;
    
    // Test TrOCR
    let mut trocr = SimpleTrOCR::new()?;
    let result = trocr.extract_text(&buffer).await?;
    
    println!("\nResult: {}", result);
    println!("\n✅ TrOCR integration test complete!");
    
    Ok(())
}