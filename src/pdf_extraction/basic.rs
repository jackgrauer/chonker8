// SPATIALLY ACCURATE PDF TEXT EXTRACTION
use anyhow::Result;
use pdfium_render::prelude::*;
use std::path::Path;
use super::pdfium_singleton::with_pdfium;
// Removed unused config import

pub async fn extract_to_matrix(
    pdf_path: &Path,
    page_num: usize,
    width: usize,
    height: usize,
) -> Result<Vec<Vec<char>>> {
    with_pdfium(|pdfium| {
        let document = pdfium.load_pdf_from_file(pdf_path, None)?;
    let page = document.pages().get(page_num as u16)?;
    
    // Create empty grid (cache-friendly initialization)
    let mut grid = vec![vec![' '; width]; height];
    
    // Get all text from page
    let text_page = page.text()?;
    
    // Get character positions for spatial mapping
    let chars = text_page.chars();
    let page_width = page.width().value;
    let page_height = page.height().value;
    
    // Collect all characters with their positions
    let mut char_positions = Vec::new();
    let mut char_count = 0;
    for char_info in chars.iter() {
        char_count += 1;
        if let Ok(bounds) = char_info.loose_bounds() {
            if let Some(ch) = char_info.unicode_string() {
                if let Some(first_char) = ch.chars().next() {
                    let x = bounds.left().value;
                    let y = bounds.top().value;
                    char_positions.push((first_char, x, y));
                }
            }
        }
    }
    
    
    // Sort by y position (top to bottom), then x position (left to right)
    char_positions.sort_by(|a, b| {
        let y_cmp = a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal);
        if y_cmp == std::cmp::Ordering::Equal {
            a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            y_cmp
        }
    });
    
    // Place characters on the grid with better spacing
    for (ch, x, y) in char_positions {
        if ch != ' ' && ch != '\n' && ch != '\r' {
            let grid_x = ((x / page_width) * width as f32).round() as usize;
            let grid_y = ((y / page_height) * height as f32).round() as usize;
            
            if grid_x < width && grid_y < height {
                grid[grid_y][grid_x] = ch;
            }
        }
    }
    
        Ok(grid)
    })
}

pub async fn extract_with_ml(_pdf_path: &Path, _page_num: usize, width: usize, height: usize) -> Result<Vec<Vec<char>>> {
    // ML removed - just return empty grid
    Ok(vec![vec![' '; width]; height])
}

pub fn get_page_count(pdf_path: &Path) -> Result<usize> {
    with_pdfium(|pdfium| {
        let document = pdfium.load_pdf_from_file(pdf_path, None)?;
        Ok(document.pages().len() as usize)
    })
}

// Simple text extraction for router
pub async fn extract_with_pdfium(pdf_path: &Path, page_index: usize) -> Result<String> {
    extract_with_pdfium_sync(pdf_path, page_index)
}

// Synchronous version to avoid async runtime issues
pub fn extract_with_pdfium_sync(pdf_path: &Path, page_index: usize) -> Result<String> {
    eprintln!("[DEBUG] extract_with_pdfium_sync: Starting extraction for {:?} page {}", pdf_path, page_index);
    
    with_pdfium(|pdfium| {
        eprintln!("[DEBUG] Got Pdfium singleton");
        
        eprintln!("[DEBUG] Loading PDF document...");
        let document = pdfium.load_pdf_from_file(pdf_path, None)?;
        eprintln!("[DEBUG] Document loaded, getting page {}...", page_index);
        
        let page = document.pages().get(page_index as u16)?;
        eprintln!("[DEBUG] Got page, extracting text...");
        
        let text_page = page.text()?;
        eprintln!("[DEBUG] Got text page object, calling all()...");
        
        let text = text_page.all();
        eprintln!("[DEBUG] Text extraction complete, {} chars", text.len());
        
        Ok(text)
    })
}