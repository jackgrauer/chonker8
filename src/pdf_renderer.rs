// PDF rendering module - renders PDF pages as images
use anyhow::Result;
use image::DynamicImage;
use pdfium_render::prelude::*;
use std::path::Path;

use crate::debug_log;

pub fn render_pdf_page(pdf_path: &Path, page_num: usize, width: u32, height: u32) -> Result<DynamicImage> {
    debug_log(format!("Rendering PDF page {} from {:?}", page_num, pdf_path));
    
    // Initialize PDFium
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./lib/"))
            .or_else(|_| Pdfium::bind_to_system_library())?
    );
    
    // Load the PDF document
    let document = pdfium.load_pdf_from_file(pdf_path, None)?;
    
    // Get the specified page
    let page = document.pages().get(page_num.try_into().unwrap())?;
    
    // Create render configuration
    let render_config = PdfRenderConfig::new()
        .set_target_size(width as i32, height as i32)
        .set_maximum_width(width as i32)
        .set_maximum_height(height as i32)
        .rotate_if_landscape(PdfPageRenderRotation::None, true);
    
    // Render the page to an image
    let bitmap = page.render_with_config(&render_config)?;
    let image = bitmap.as_image();
    
    debug_log(format!("Page rendered: {}x{} pixels", image.width(), image.height()));
    
    Ok(image)
}