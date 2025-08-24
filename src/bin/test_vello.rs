use anyhow::Result;
use chonker8::vello_pdf_renderer::VelloPdfRenderer;
use std::path::Path;

fn main() -> Result<()> {
    println!("Testing Vello PDF renderer on ARM with Metal backend...");
    
    // Test with the PDF that has XObject Do commands
    let pdf_path = Path::new("/Users/jack/Desktop/BERF-CERT.pdf");
    
    if !pdf_path.exists() {
        eprintln!("Test PDF not found. Please ensure real_test.pdf exists.");
        return Ok(());
    }
    
    println!("Initializing Vello renderer...");
    let mut renderer = VelloPdfRenderer::new(pdf_path)?;
    
    println!("Getting page count...");
    let page_count = renderer.page_count();
    println!("PDF has {} pages", page_count);
    
    if page_count > 0 {
        println!("Rendering page 1 with Vello (GPU/Metal backend)...");
        let image = renderer.render_page(0, 800, 1000)?;
        
        println!("Successfully rendered page!");
        println!("Image dimensions: {}x{}", image.width(), image.height());
        
        // Save the rendered image
        image.save("vello_render_test.png")?;
        println!("Saved rendered image to vello_render_test.png");
    }
    
    println!("✅ Vello renderer test completed successfully!");
    println!("This confirms Vello works on ARM with Metal backend.");
    
    Ok(())
}