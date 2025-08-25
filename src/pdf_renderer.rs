use anyhow::Result;
use image::DynamicImage;
use std::path::Path;

// Use our Vello renderer
use crate::vello_pdf_renderer::VelloPdfRenderer;

/// Render a PDF page to an image using Vello (GPU-accelerated on ARM/Metal)
pub fn render_pdf_page(pdf_path: &Path, page_num: usize, width: u32, height: u32) -> Result<DynamicImage> {
    // Render PDF page using Vello
    // Create a Vello renderer instance
    let mut renderer = VelloPdfRenderer::new(pdf_path)?;
    
    // Render the requested page
    renderer.render_page(page_num, width, height)
}

/// Get the total number of pages in a PDF using Vello renderer
pub fn get_pdf_page_count(pdf_path: &Path) -> Result<usize> {
    let renderer = VelloPdfRenderer::new(pdf_path)?;
    Ok(renderer.page_count())
}
