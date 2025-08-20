// Improved PDF text extraction that preserves word formation
use anyhow::Result;
use pdfium_render::prelude::*;
use std::path::Path;

pub async fn extract_with_word_grouping(
    pdf_path: &Path,
    page_index: usize,
    width: usize,
    height: usize,
) -> Result<Vec<Vec<char>>> {
    // Initialize PDFium
    let lib_path = crate::config::pdfium_library_path();
    let bindings = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&lib_path))?
    );
    
    // Load the PDF
    let document = bindings.load_pdf_from_file(pdf_path, None)?;
    let page = document.pages().get(page_index as u16)?;
    
    // Get page dimensions
    let page_width = page.width().value;
    let page_height = page.height().value;
    
    // Extract text with better word grouping
    let text_page = page.text()?;
    
    // Initialize the grid
    let mut grid = vec![vec![' '; width]; height];
    
    // Collect all characters with positions
    let mut char_data = Vec::new();
    for char_info in text_page.chars().iter() {
        if let Ok(bounds) = char_info.loose_bounds() {
            if let Some(ch) = char_info.unicode_string() {
                if let Some(character) = ch.chars().next() {
                    if character != ' ' && character != '\n' && character != '\r' {
                        char_data.push((
                            character,
                            bounds.left().value,
                            bounds.top().value,
                            bounds.right().value,
                            bounds.bottom().value
                        ));
                    }
                }
            }
        }
    }
    
    // Sort by y position - PDF Y increases from bottom to top
    // So for top-to-bottom reading, we want DECREASING Y values
    char_data.sort_by(|a, b| {
        // First sort by Y (descending - higher Y first since that's the top of the page)
        let y_cmp = b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal);
        if y_cmp == std::cmp::Ordering::Equal {
            // Then by X (ascending - left to right)
            a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            y_cmp
        }
    });
    
    // Group characters into words based on proximity
    let mut words = Vec::new();
    let mut current_word = String::new();
    let mut word_start_x = 0.0;
    let mut word_start_y = 0.0;
    let mut last_right = -100.0;
    let mut last_y = -100.0;
    
    for (ch, left, top, right, _bottom) in char_data {
        // Check if this character is close to the last one
        let x_gap = left - last_right;
        let y_distance = (top - last_y).abs();
        
        // Threshold for same word: very small gap horizontally, same line vertically
        // Typical character width is 5-10 points, so gap should be less than 3 points for same word
        let is_same_word = x_gap < 3.0 && x_gap > -1.0 && y_distance < 2.0;
        
        if is_same_word && !current_word.is_empty() {
            // Add to current word
            current_word.push(ch);
        } else {
            // Save previous word if exists
            if !current_word.is_empty() {
                words.push((current_word.clone(), word_start_x, word_start_y));
            }
            // Start new word
            current_word = ch.to_string();
            word_start_x = left;
            word_start_y = top;
        }
        
        last_right = right;
        last_y = top;
    }
    
    // Don't forget the last word
    if !current_word.is_empty() {
        words.push((current_word, word_start_x, word_start_y));
    }
    
    // Place words on the grid with proper spacing
    for (word, x, y) in words {
        let grid_y = height - 1 - ((y / page_height) * height as f32).round() as usize;
        let start_x = ((x / page_width) * width as f32).round() as usize;
        
        if grid_y < height {
            // Place each character of the word
            let mut placed_chars = 0;
            for (i, ch) in word.chars().enumerate() {
                let grid_x = start_x + i;
                if grid_x < width {
                    grid[grid_y][grid_x] = ch;
                    placed_chars += 1;
                }
            }
            
            // Add a space after the word if there's room
            let space_x = start_x + placed_chars;
            if space_x < width && grid[grid_y][space_x] == ' ' {
                // Space is already there, good
            }
        }
    }
    
    Ok(grid)
}
