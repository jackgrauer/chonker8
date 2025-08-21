use anyhow::Result;
use image::DynamicImage;
use crossterm::{execute, style::{Print, SetForegroundColor, ResetColor, Color}, cursor::MoveTo};
use std::io::stdout;

/// Convert an image to ASCII art for terminal display
pub fn display_pdf_as_ascii(
    image: &DynamicImage,
    x: u16,
    y: u16,
    max_width: u16,
    max_height: u16,
) -> Result<()> {
    let ascii_chars = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
    
    // Convert to grayscale and resize to fit terminal
    let gray_image = image.to_luma8();
    let (img_width, img_height) = (gray_image.width(), gray_image.height());
    
    // Calculate display dimensions
    let display_width = (max_width as u32).min(img_width / 8).min(80) as u16; // Scale down
    let display_height = (max_height as u32).min(img_height / 16).min(40) as u16; // Scale down more for height
    
    // Sample pixels and convert to ASCII
    for row in 0..display_height {
        execute!(stdout(), MoveTo(x, y + row))?;
        
        let mut line = String::new();
        for col in 0..display_width {
            // Sample pixel from original image
            let sample_x = (col as u32 * img_width / display_width as u32).min(img_width - 1);
            let sample_y = (row as u32 * img_height / display_height as u32).min(img_height - 1);
            
            let pixel = gray_image.get_pixel(sample_x, sample_y);
            let brightness = pixel[0] as usize;
            
            // Convert brightness to ASCII character (invert for PDF on white background)
            let char_index = (255 - brightness) * (ascii_chars.len() - 1) / 255;
            let ascii_char = ascii_chars[char_index];
            
            line.push(ascii_char);
        }
        
        execute!(
            stdout(),
            SetForegroundColor(Color::White),
            Print(&line),
            ResetColor
        )?;
    }
    
    Ok(())
}