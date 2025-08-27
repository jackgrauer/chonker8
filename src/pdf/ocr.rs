// OCR support for scanned PDFs using Tesseract
use anyhow::{Result, anyhow};
use image::DynamicImage;
use rusty_tesseract::{Image, Args};

/// Perform OCR on a PDF page image to extract text
pub fn ocr_image(image: &DynamicImage) -> Result<String> {
    // Convert DynamicImage to rusty_tesseract Image
    let img = Image::from_dynamic_image(image)
        .map_err(|e| anyhow!("Failed to convert image for OCR: {}", e))?;
    
    // Set up Tesseract arguments
    let mut args = Args::default();
    
    // Use English language (you can make this configurable)
    args.lang = "eng".to_string();
    
    // Set page segmentation mode to automatic
    args.psm = Some(3); // PSM 3: Fully automatic page segmentation
    
    // Set OCR engine mode to use LSTM neural nets (best accuracy)
    args.oem = Some(3); // OEM 3: Default, based on what's available
    
    // Perform OCR
    let output = rusty_tesseract::image_to_string(&img, &args)
        .map_err(|e| anyhow!("OCR failed: {}", e))?;
    
    Ok(output)
}

/// Perform OCR with layout preservation for grid display
pub fn ocr_image_with_layout(image: &DynamicImage, width: usize, height: usize) -> Result<Vec<Vec<char>>> {
    // For now, just get the text and format it into a grid
    // rusty-tesseract doesn't expose detailed position data in a simple way
    let text = ocr_image(image)?;
    
    // Create empty grid
    let mut grid = vec![vec![' '; width]; height];
    
    // Simple layout: split text into lines and place them
    let lines: Vec<&str> = text.lines().collect();
    for (y, line) in lines.iter().enumerate() {
        if y >= height {
            break;
        }
        for (x, ch) in line.chars().enumerate() {
            if x >= width {
                break;
            }
            grid[y][x] = ch;
        }
    }
    
    Ok(grid)
}

/// Check if a text extraction result likely needs OCR
pub fn needs_ocr(extracted_text: &str) -> bool {
    // Consider it needs OCR if:
    // 1. Text is empty or very short
    // 2. Text is mostly whitespace
    let cleaned = extracted_text.trim();
    
    if cleaned.is_empty() {
        return true;
    }
    
    // Check if text length is suspiciously short for a full page
    if cleaned.len() < 50 {
        return true;
    }
    
    // Check if the text has very few actual words
    let word_count = cleaned.split_whitespace().count();
    if word_count < 10 {
        return true;
    }
    
    false
}

/// Get available Tesseract languages
pub fn get_available_languages() -> Vec<String> {
    // This would need to check the tessdata directory
    // For now, return common languages
    vec![
        "eng".to_string(),  // English
        "spa".to_string(),  // Spanish  
        "fra".to_string(),  // French
        "deu".to_string(),  // German
        "ita".to_string(),  // Italian
        "por".to_string(),  // Portuguese
        "rus".to_string(),  // Russian
        "jpn".to_string(),  // Japanese
        "chi_sim".to_string(), // Simplified Chinese
        "chi_tra".to_string(), // Traditional Chinese
    ]
}