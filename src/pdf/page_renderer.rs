use anyhow::Result;
use image::DynamicImage;
use std::path::Path;

// Use system's pdftoppm for ACTUAL working PDF rendering
use crate::system_pdf_renderer::SystemPdfRenderer;

/// Render a PDF page to an image using the system's pdftoppm
pub fn render_pdf_page(pdf_path: &Path, page_num: usize, width: u32, height: u32) -> Result<DynamicImage> {
    eprintln!("[PDF_RENDERER] Using system pdftoppm for PDF rendering");
    
    // Create system renderer
    let renderer = SystemPdfRenderer::new();
    
    // Render to bitmap using pdftoppm
    let image = renderer.render_page_to_bitmap(pdf_path, page_num, width, height)?;
    
    eprintln!("[PDF_RENDERER] ✅ Page rendered to bitmap successfully");
    Ok(image)
}

/// Get the total number of pages in a PDF using pdfinfo
pub fn get_pdf_page_count(pdf_path: &Path) -> Result<usize> {
    use std::process::Command;
    
    let output = Command::new("pdfinfo")
        .arg(pdf_path)
        .output()?;
        
    if !output.status.success() {
        // Fallback to lopdf if pdfinfo isn't available
        use lopdf::Document;
        let document = Document::load(pdf_path)?;
        return Ok(document.get_pages().len());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.starts_with("Pages:") {
            if let Some(count_str) = line.split(':').nth(1) {
                if let Ok(count) = count_str.trim().parse::<usize>() {
                    return Ok(count);
                }
            }
        }
    }
    
    // Fallback if we couldn't parse pdfinfo output
    use lopdf::Document;
    let document = Document::load(pdf_path)?;
    Ok(document.get_pages().len())
}
