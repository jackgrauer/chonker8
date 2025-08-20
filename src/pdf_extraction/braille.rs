// Braille-based high-fidelity PDF rendering for terminal display
use anyhow::Result;
use pdfium_render::prelude::*;

// Braille patterns for 2x4 sub-character resolution
// Each u16 represents which dots are filled in the 2x4 grid:
// ⠁⠂⠃⠄⠅⠆⠇⠈⠉⠊⠋⠌⠍⠎⠏⠐⠑⠒⠓⠔⠕⠖⠗⠘⠙⠚⠛⠜⠝⠞⠟⠠⠡⠢⠣⠤⠥⠦⠧⠨⠩⠪⠫⠬⠭⠮⠯⠰⠱⠲⠳⠴⠵⠶⠷⠸⠹⠺⠻⠼⠽⠾⠿
// Dot positions in 2x4 grid:
// 1 4
// 2 5  
// 3 6
// 7 8

const BRAILLE_BASE: u32 = 0x2800; // Unicode base for Braille patterns

/// Convert a 2x4 pixel grid to a Braille character
fn pixels_to_braille(pixels: [[bool; 2]; 4]) -> char {
    let mut pattern = 0u8;
    
    // Map pixel positions to Braille dot positions
    if pixels[0][0] { pattern |= 0x01; } // dot 1
    if pixels[1][0] { pattern |= 0x02; } // dot 2  
    if pixels[2][0] { pattern |= 0x04; } // dot 3
    if pixels[0][1] { pattern |= 0x08; } // dot 4
    if pixels[1][1] { pattern |= 0x10; } // dot 5
    if pixels[2][1] { pattern |= 0x20; } // dot 6
    if pixels[3][0] { pattern |= 0x40; } // dot 7
    if pixels[3][1] { pattern |= 0x80; } // dot 8
    
    char::from_u32(BRAILLE_BASE + pattern as u32).unwrap_or(' ')
}

/// Render a PDF page at high resolution and downsample to Braille characters
pub async fn extract_to_braille(
    pdf_path: &std::path::Path,
    page_index: usize,
    terminal_width: usize,
    terminal_height: usize,
) -> Result<Vec<Vec<char>>> {
    // Calculate high-resolution dimensions
    // Each terminal character represents 2x4 pixels in Braille
    // We want to render at 16x for sharpness
    let render_width = terminal_width * 2 * 16;  // 2 columns per char * 16x oversampling
    let render_height = terminal_height * 4 * 16; // 4 rows per char * 16x oversampling
    
    // Initialize PDFium
    let bindings = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./lib/"))
            .or_else(|_| Pdfium::bind_to_system_library())?,
    );
    
    // Load the PDF
    let document = bindings.load_pdf_from_file(pdf_path, None)?;
    let page = document.pages().get(page_index)?;
    
    // Get page dimensions
    let page_width = page.width();
    let page_height = page.height();
    
    // Calculate scale to fit the page into our render dimensions
    let scale_x = render_width as f32 / page_width.value;
    let scale_y = render_height as f32 / page_height.value;
    let scale = scale_x.min(scale_y); // Use uniform scale to maintain aspect ratio
    
    // Create a bitmap for rendering
    let actual_render_width = (page_width.value * scale) as i32;
    let actual_render_height = (page_height.value * scale) as i32;
    
    // Use page's render method to create bitmap
    let bitmap = page.render_with_config(&PdfRenderConfig::new()
        .set_target_size(actual_render_width, actual_render_height)
        .render_annotations(true))?;
    
    // Convert bitmap to pixel array
    let pixels = bitmap_to_pixels(&bitmap, actual_render_width, actual_render_height)?;
    
    // Downsample to Braille characters
    let mut braille_grid = vec![vec![' '; terminal_width]; terminal_height];
    
    // Size of each block in the high-res image that maps to one Braille character
    let block_width = actual_render_width as usize / terminal_width;
    let block_height = actual_render_height as usize / terminal_height;
    
    for row in 0..terminal_height {
        for col in 0..terminal_width {
            // Extract the 2x4 sub-blocks for this character position
            let mut sub_pixels = [[false; 2]; 4];
            
            // Each sub-block represents 1/8th of the full block
            let sub_width = block_width / 2;
            let sub_height = block_height / 4;
            
            // Sample each sub-block
            for sub_row in 0..4 {
                for sub_col in 0..2 {
                    let start_x = col * block_width + sub_col * sub_width;
                    let start_y = row * block_height + sub_row * sub_height;
                    
                    // Count black pixels in this sub-block
                    let mut black_count = 0;
                    let mut total_count = 0;
                    
                    for y in start_y..start_y.min(start_y + sub_height).min(pixels.len()) {
                        for x in start_x..start_x.min(start_x + sub_width).min(pixels[0].len()) {
                            total_count += 1;
                            if pixels[y][x] {
                                black_count += 1;
                            }
                        }
                    }
                    
                    // If more than 25% of pixels are black, consider this sub-block filled
                    sub_pixels[sub_row][sub_col] = black_count > total_count / 4;
                }
            }
            
            braille_grid[row][col] = pixels_to_braille(sub_pixels);
        }
    }
    
    Ok(braille_grid)
}

/// Convert bitmap to boolean pixel array (true = black, false = white)
fn bitmap_to_pixels(
    bitmap: &PdfBitmap,
    width: i32,
    height: i32,
) -> Result<Vec<Vec<bool>>> {
    let mut pixels = vec![vec![false; width as usize]; height as usize];
    
    for y in 0..height {
        for x in 0..width {
            // Get pixel color (BGR format)
            let pixel = bitmap.get_pixel(x, y);
            
            // Convert to grayscale
            let gray = (pixel.red() as f32 * 0.299 + 
                       pixel.green() as f32 * 0.587 + 
                       pixel.blue() as f32 * 0.114) as u8;
            
            // Threshold: consider dark pixels as black
            pixels[y as usize][x as usize] = gray < 128;
        }
    }
    
    Ok(pixels)
}

/// Different rendering modes
pub enum RenderMode {
    Draft,      // Simple dots for layout
    Standard,   // Half-block characters
    HighFidelity, // Full Braille resolution
}

/// Extract with specified rendering mode
pub async fn extract_with_mode(
    pdf_path: &std::path::Path,
    page_index: usize,
    width: usize,
    height: usize,
    mode: RenderMode,
) -> Result<Vec<Vec<char>>> {
    match mode {
        RenderMode::Draft => {
            // Simple dot rendering - just show where text exists
            super::basic::extract_to_matrix(pdf_path, page_index, width, height).await
        },
        RenderMode::Standard => {
            // Half-block rendering (▀▄█▌▐░▒▓)
            // TODO: Implement half-block rendering
            extract_to_braille(pdf_path, page_index, width, height).await
        },
        RenderMode::HighFidelity => {
            // Full Braille resolution
            extract_to_braille(pdf_path, page_index, width, height).await
        },
    }
}