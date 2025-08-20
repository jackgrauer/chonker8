// Simpler visual ground truth using page rendering
use anyhow::Result;
use pdfium_render::prelude::*;
use std::path::Path;

/// Render PDF page to get actual visual layout
pub async fn render_true_visual(
    pdf_path: &Path,
    page_index: usize,
    term_width: usize,
    term_height: usize,
) -> Result<Vec<Vec<char>>> {
    // Initialize PDFium
    let lib_path = crate::config::pdfium_library_path();
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&lib_path))?
    );
    
    // Load the PDF
    let document = pdfium.load_pdf_from_file(pdf_path, None)?;
    let page = document.pages().get(page_index as u16)?;
    
    // Get the text objects and their positions to create ground truth
    let text_page = page.text()?;
    let page_width = page.width().value;
    let page_height = page.height().value;
    
    // Create grid showing where text actually is
    let mut grid = vec![vec![' '; term_width]; term_height];
    
    // Process each character and mark its actual position
    for char_info in text_page.chars().iter() {
        if let Ok(bounds) = char_info.loose_bounds() {
            if let Some(ch_str) = char_info.unicode_string() {
                if let Some(ch) = ch_str.chars().next() {
                    if ch != ' ' && ch != '\n' && ch != '\r' {
                        // Map PDF coordinates to terminal grid
                        // Note: PDF Y coordinates are inverted - Y=0 is at bottom
                        // So we need to flip the Y coordinate for terminal display
                        let x = ((bounds.left().value / page_width) * term_width as f32) as usize;
                        let y = term_height - 1 - ((bounds.top().value / page_height) * term_height as f32) as usize;
                        
                        if x < term_width && y < term_height {
                            // Show different symbols for different character types
                            grid[y][x] = match ch {
                                'a'..='z' => '█',  // Lowercase letters
                                'A'..='Z' => '▓',  // Uppercase letters  
                                '0'..='9' => '▒',  // Numbers
                                '.' | ',' => '░',  // Punctuation
                                _ => '▪',          // Other
                            };
                        }
                    }
                }
            }
        }
    }
    
    Ok(grid)
}