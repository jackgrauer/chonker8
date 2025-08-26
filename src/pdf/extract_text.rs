// SPATIALLY ACCURATE PDF TEXT EXTRACTION - Pure Rust Implementation
use anyhow::{Result, anyhow};
use lopdf::{Document, Object, Dictionary};
use std::path::Path;

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
    
    // Group characters into words for better formatting
    let words = group_characters_into_words(&char_positions);
    
    // Place words on grid with better spacing
    for word in words {
        if word.chars.is_empty() {
            continue;
        }
        
        // Calculate word position on grid
        let start_x = ((word.x / page_width) * width as f32) as usize;
        let y = (((page_height - word.y) / page_height) * height as f32) as usize;
        
        // Place the entire word
        if y < height {
            let mut x = start_x;
            for ch in word.chars.chars() {
                if x < width {
                    grid[y][x] = ch;
                    x += 1;
                }
            }
        }
    }
    
    // Post-process to clean up formatting
    clean_up_grid(&mut grid);
    
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

// Structure to hold word information
struct Word {
    chars: String,
    x: f32,
    y: f32,
}

// Group characters into words based on proximity
fn group_characters_into_words(char_positions: &[(char, f32, f32)]) -> Vec<Word> {
    let mut words = Vec::new();
    
    if char_positions.is_empty() {
        return words;
    }
    
    let mut current_word = String::new();
    let mut word_x = 0.0;
    let mut word_y = 0.0;
    let mut last_x = -1000.0;
    let mut last_y = -1000.0;
    
    const SPACE_THRESHOLD: f32 = 8.0;  // Space between characters to consider new word
    const LINE_THRESHOLD: f32 = 10.0;  // Vertical distance to consider new line
    
    for &(ch, x, y) in char_positions {
        // Check if this is a new word or line
        let is_new_word = (x - last_x).abs() > SPACE_THRESHOLD || 
                          (y - last_y).abs() > LINE_THRESHOLD;
        
        if is_new_word && !current_word.is_empty() {
            // Save the current word
            words.push(Word {
                chars: current_word.clone(),
                x: word_x,
                y: word_y,
            });
            current_word.clear();
        }
        
        if current_word.is_empty() {
            // Start of new word
            word_x = x;
            word_y = y;
        }
        
        current_word.push(ch);
        last_x = x;
        last_y = y;
    }
    
    // Don't forget the last word
    if !current_word.is_empty() {
        words.push(Word {
            chars: current_word,
            x: word_x,
            y: word_y,
        });
    }
    
    words
}

// Clean up the grid to improve formatting
fn clean_up_grid(grid: &mut Vec<Vec<char>>) {
    let height = grid.len();
    let width = if height > 0 { grid[0].len() } else { 0 };
    
    // Remove isolated characters (likely noise)
    for y in 0..height {
        for x in 0..width {
            if grid[y][x] != ' ' {
                // Check if character is isolated
                let mut neighbor_count = 0;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let ny = (y as i32 + dy) as usize;
                        let nx = (x as i32 + dx) as usize;
                        if ny < height && nx < width && grid[ny][nx] != ' ' {
                            neighbor_count += 1;
                        }
                    }
                }
                // If character has no neighbors, it's likely noise
                if neighbor_count == 0 && !grid[y][x].is_alphanumeric() {
                    grid[y][x] = ' ';
                }
            }
        }
    }
    
    // Trim trailing spaces from each line
    for row in grid.iter_mut() {
        let mut last_char = row.len();
        for (i, &ch) in row.iter().enumerate().rev() {
            if ch != ' ' {
                last_char = i + 1;
                break;
            }
        }
        for i in last_char..row.len() {
            row[i] = ' ';
        }
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