// SPATIALLY ACCURATE PDF TEXT EXTRACTION - Pure Rust Implementation
use anyhow::{Result, anyhow};
use lopdf::{Document, Object, Dictionary};
use std::path::Path;
use std::collections::BTreeMap;

pub async fn extract_to_matrix(
    pdf_path: &Path,
    page_num: usize,
    width: usize,
    height: usize,
) -> Result<Vec<Vec<char>>> {
    // Load PDF with lopdf
    let document = Document::load(pdf_path)?;
    
    // Create empty grid
    let mut grid = vec![vec![' '; width]; height];
    
    // Get the page
    let pages = document.get_pages();
    let page_id = pages
        .get(&(page_num as u32 + 1))
        .ok_or_else(|| anyhow!("Page {} not found", page_num + 1))?;
    
    // Get page dimensions
    let page_dict = document.get_object(*page_id)?
        .as_dict()?;
    
    let media_box = get_media_box(&document, page_dict)?;
    let page_width = media_box[2] - media_box[0];
    let page_height = media_box[3] - media_box[1];
    
    // Extract text with positions
    let char_positions = extract_text_with_positions(&document, page_dict)?;
    
    // Map characters to grid positions
    for (ch, x, y) in char_positions {
        // Convert PDF coordinates to grid coordinates
        // PDF coordinates are bottom-left origin, so flip y
        let grid_x = ((x / page_width) * width as f32) as usize;
        let grid_y = (((page_height - y) / page_height) * height as f32) as usize;
        
        // Clamp to grid bounds
        if grid_x < width && grid_y < height {
            grid[grid_y][grid_x] = ch;
        }
    }
    
    Ok(grid)
}

pub fn get_page_count(pdf_path: &Path) -> Result<usize> {
    let document = Document::load(pdf_path)?;
    Ok(document.get_pages().len())
}

// Helper function to get media box dimensions
fn get_media_box(document: &Document, page: &Dictionary) -> Result<Vec<f32>> {
    if let Ok(media_box) = page.get(b"MediaBox") {
        let arr = match media_box {
            Object::Reference(id) => {
                if let Ok(Object::Array(a)) = document.get_object(*id) {
                    a
                } else {
                    return Ok(vec![0.0, 0.0, 612.0, 792.0]);
                }
            }
            Object::Array(a) => a,
            _ => return Ok(vec![0.0, 0.0, 612.0, 792.0]),
        };
        
        let mut bounds = Vec::new();
        for obj in arr {
            match obj {
                Object::Integer(i) => bounds.push(*i as f32),
                Object::Real(f) => bounds.push(*f),
                _ => {}
            }
        }
        if bounds.len() == 4 {
            return Ok(bounds);
        }
    }
    
    // Default to US Letter if no MediaBox
    Ok(vec![0.0, 0.0, 612.0, 792.0])
}

// Extract text with positions from page
fn extract_text_with_positions(document: &Document, page: &Dictionary) -> Result<Vec<(char, f32, f32)>> {
    let mut char_positions = Vec::new();
    
    // Get content streams
    let contents = page.get(b"Contents")?;
    let content_data = get_content_data(document, contents)?;
    
    // Parse content stream for text operations
    let mut current_x = 0.0;
    let mut current_y = 0.0;
    let mut text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]; // Identity matrix
    
    // Simple content stream parser for text operations
    let content_str = String::from_utf8_lossy(&content_data);
    let lines: Vec<&str> = content_str.lines().collect();
    
    for line in lines {
        let line = line.trim();
        
        // Text positioning operators
        if line.ends_with(" Td") {
            // Text position
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                if let (Ok(tx), Ok(ty)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                    current_x += tx;
                    current_y += ty;
                }
            }
        } else if line.ends_with(" Tm") {
            // Text matrix
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 7 {
                for i in 0..6 {
                    if let Ok(val) = parts[i].parse::<f32>() {
                        text_matrix[i] = val;
                    }
                }
                current_x = text_matrix[4];
                current_y = text_matrix[5];
            }
        } else if line.contains("Tj") {
            // Show text string
            if let Some(text) = extract_text_from_tj(line) {
                // Add each character with current position
                for (i, ch) in text.chars().enumerate() {
                    // Simple character spacing approximation
                    let char_x = current_x + (i as f32 * 6.0); // Approximate char width
                    char_positions.push((ch, char_x, current_y));
                }
            }
        } else if line.contains("TJ") {
            // Show text with individual glyph positioning
            if let Some(text) = extract_text_from_tj_array(line) {
                for (i, ch) in text.chars().enumerate() {
                    let char_x = current_x + (i as f32 * 6.0);
                    char_positions.push((ch, char_x, current_y));
                }
            }
        }
    }
    
    // Sort by y position (top to bottom), then x position (left to right)
    char_positions.sort_by(|a, b| {
        let y_cmp = b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal);
        if y_cmp == std::cmp::Ordering::Equal {
            a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            y_cmp
        }
    });
    
    Ok(char_positions)
}

// Get content data from content object
fn get_content_data(document: &Document, contents: &Object) -> Result<Vec<u8>> {
    match contents {
        Object::Reference(r) => {
            let obj = document.get_object(*r)?;
            get_content_data(document, obj)
        }
        Object::Stream(stream) => {
            Ok(stream.decompressed_content()?)
        }
        Object::Array(arr) => {
            let mut data = Vec::new();
            for item in arr {
                let item_data = get_content_data(document, item)?;
                data.extend_from_slice(&item_data);
            }
            Ok(data)
        }
        _ => Ok(Vec::new())
    }
}

// Extract text from Tj operator
fn extract_text_from_tj(line: &str) -> Option<String> {
    // Find text between parentheses
    if let Some(start) = line.find('(') {
        if let Some(end) = line.rfind(')') {
            if end > start {
                let text = &line[start + 1..end];
                // Basic PDF string decoding
                return Some(decode_pdf_string(text));
            }
        }
    }
    None
}

// Extract text from TJ array operator
fn extract_text_from_tj_array(line: &str) -> Option<String> {
    // Find text between brackets
    if let Some(start) = line.find('[') {
        if let Some(end) = line.rfind(']') {
            if end > start {
                let array_content = &line[start + 1..end];
                let mut result = String::new();
                
                // Extract strings from array
                let mut in_string = false;
                let mut current_string = String::new();
                
                for ch in array_content.chars() {
                    if ch == '(' {
                        in_string = true;
                        current_string.clear();
                    } else if ch == ')' && in_string {
                        in_string = false;
                        result.push_str(&decode_pdf_string(&current_string));
                    } else if in_string {
                        current_string.push(ch);
                    }
                }
                
                if !result.is_empty() {
                    return Some(result);
                }
            }
        }
    }
    None
}

// Basic PDF string decoder
fn decode_pdf_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            // Handle escape sequences
            if let Some(next) = chars.next() {
                match next {
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    '\\' => result.push('\\'),
                    '(' => result.push('('),
                    ')' => result.push(')'),
                    _ => {
                        // Octal escape or just add the character
                        result.push(next);
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}