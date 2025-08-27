use anyhow::Result;
use image::DynamicImage;
use std::path::Path;

// Use system's pdftoppm for ACTUAL working PDF rendering
use crate::pdf::render_with_pdftoppm::SystemPdfRenderer;

/// Render a PDF page to an image using the system's pdftoppm
pub fn render_pdf_page(pdf_path: &Path, page_num: usize, width: u32, height: u32) -> Result<DynamicImage> {
    // eprintln!("[PDF_RENDERER] Using system pdftoppm for PDF rendering");
    
    // Create system renderer
    let renderer = SystemPdfRenderer::new();
    
    // Render to bitmap using pdftoppm
    let image = renderer.render_page_to_bitmap(pdf_path, page_num, width, height)?;
    
    // eprintln!("[PDF_RENDERER] ✅ Page rendered to bitmap successfully");
    Ok(image)
}

/// Get the total number of pages in a PDF using pdfinfo
pub fn get_pdf_page_count(pdf_path: &Path) -> Result<usize> {
    use std::process::Command;
    
    // For large files, add a timeout using the timeout command
    let output = Command::new("timeout")
        .arg("10")  // 10 second timeout
        .arg("pdfinfo")
        .arg(pdf_path)
        .output()?;
        
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.code() == Some(124) {
            return Err(anyhow::anyhow!("pdfinfo timed out after 10s (PDF file may be too large)"));
        }
        return Err(anyhow::anyhow!("pdfinfo failed (exit code {:?}): {}", output.status.code(), stderr.trim()));
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
    
    // If we couldn't parse the output, assume 1 page
    Ok(1)
}
