use anyhow::Result;
use chonker8::pdf_renderer;
use std::path::Path;

fn main() -> Result<()> {
    println!("Testing lopdf-vello pipeline (PDFium removed!)...");
    
    let pdf_path = Path::new("real_test.pdf");
    
    if !pdf_path.exists() {
        eprintln!("Test PDF not found. Please ensure real_test.pdf exists.");
        return Ok(());
    }
    
    // Test page count with lopdf
    println!("Getting page count with lopdf...");
    let page_count = pdf_renderer::get_pdf_page_count(pdf_path)?;
    println!("✓ PDF has {} pages (using lopdf)", page_count);
    
    // Test rendering with Vello
    println!("Rendering page 1 with Vello GPU acceleration...");
    let image = pdf_renderer::render_pdf_page(pdf_path, 0, 800, 1000)?;
    println!("✓ Successfully rendered page with Vello!");
    println!("  Image dimensions: {}x{}", image.width(), image.height());
    
    // Save test image
    image.save("lopdf_vello_test.png")?;
    println!("✓ Saved test image to lopdf_vello_test.png");
    
    println!();
    println!("🎉 lopdf-vello-kitty pipeline test successful!");
    println!("   PDFium has been completely removed!");
    println!("   Using pure Rust: lopdf for parsing, Vello for GPU rendering");
    
    Ok(())
}
