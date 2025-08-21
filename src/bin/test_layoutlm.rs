use anyhow::Result;
use std::path::Path;

#[path = "../pdf_extraction/document_understanding.rs"]
mod document_understanding;
use document_understanding::{DocumentAnalyzer, DocumentType};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing LayoutLMv3 Integration\n");
    
    // Create test image
    let mut img = image::RgbImage::new(224, 224);
    for pixel in img.pixels_mut() {
        *pixel = image::Rgb([255, 255, 255]); // White background
    }
    
    // Add some structure (boxes to simulate document layout)
    // Header area
    for x in 10..214 {
        for y in 10..40 {
            img.put_pixel(x, y, image::Rgb([200, 200, 200]));
        }
    }
    
    // Text areas
    for x in 10..214 {
        for y in 50..60 {
            img.put_pixel(x, y, image::Rgb([100, 100, 100]));
        }
        for y in 70..80 {
            img.put_pixel(x, y, image::Rgb([100, 100, 100]));
        }
    }
    
    // Create DynamicImage
    let dynamic_image = image::DynamicImage::ImageRgb8(img);
    
    // Test DocumentAnalyzer
    let mut analyzer = DocumentAnalyzer::new()?;
    let test_text = "This is a test document with some sample text for LayoutLM analysis";
    let result = analyzer.analyze_document(&dynamic_image, test_text).await?;
    
    println!("\nDocument Analysis Result:");
    println!("  Sections found: {}", result.sections.len());
    if !result.sections.is_empty() {
        println!("  First section type: {:?}", result.sections[0].section_type);
    }
    
    println!("\n✅ LayoutLMv3 integration test complete!");
    
    Ok(())
}
