// System PDF renderer using pdftoppm - actually works!
use anyhow::Result;
use image::DynamicImage;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

pub struct SystemPdfRenderer;

impl SystemPdfRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn render_page_to_bitmap(&self, pdf_path: &Path, page_num: usize, width: u32, height: u32) -> Result<DynamicImage> {
        // eprintln!("[SYSTEM] Using pdftoppm to render page {} at {}x{}", page_num, width, height);
        
        // Validate inputs
        if width == 0 || height == 0 {
            return self.create_error_image(width.max(100), height.max(50), "Invalid dimensions");
        }
        
        if !pdf_path.exists() {
            return self.create_error_image(width, height, "PDF file not found");
        }
        
        // Create a temporary directory for output
        let temp_dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(_) => return self.create_error_image(width, height, "Failed to create temp dir"),
        };
        
        let output_prefix = temp_dir.path().join("page");
        
        // Use pdftoppm to convert PDF page to PNG
        // page_num is 0-based in our code but pdftoppm uses 1-based
        let page = page_num + 1;
        
        let output = match Command::new("pdftoppm")
            .args(&[
                "-png",                    // PNG format
                "-f", &page.to_string(),   // First page
                "-l", &page.to_string(),   // Last page (same as first for single page)
                "-scale-to-x", &width.to_string(),   // Scale to width
                "-scale-to-y", &height.to_string(),  // Scale to height
                pdf_path.to_str().unwrap_or(""),     // Input PDF
                output_prefix.to_str().unwrap_or(""),     // Output prefix
            ])
            .output() {
                Ok(output) => output,
                Err(e) => {
                    // pdftoppm not installed or failed to execute
                    return self.create_error_image(width, height, &format!("pdftoppm error: {}", e));
                }
            };
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Try to provide a helpful error message
            if stderr.contains("can't open file") {
                return self.create_error_image(width, height, "Cannot open PDF");
            } else if stderr.contains("Incorrect page") {
                return self.create_error_image(width, height, &format!("Page {} not found", page));
            } else {
                return self.create_error_image(width, height, "PDF render failed");
            }
        }
        
        // Find the generated PNG file
        // pdftoppm adds -1.png for single page output
        let output_file = temp_dir.path().join(format!("page-{}.png", page));
        
        if !output_file.exists() {
            // Try without page number suffix
            let alt_file = temp_dir.path().join("page-1.png");
            if alt_file.exists() {
                // eprintln!("[SYSTEM] Loading rendered page from {:?}", alt_file);
                let image = image::open(&alt_file)?;
                
                // Convert white backgrounds to pure black to match terminal
                use image::{Rgba, ImageBuffer};
                let rgba_image = image.to_rgba8();
                let (width, height) = rgba_image.dimensions();
                
                let mut black_bg_image = ImageBuffer::new(width, height);
                
                for (x, y, pixel) in rgba_image.enumerate_pixels() {
                    let Rgba([r, g, b, a]) = *pixel;
                    
                    // Full inversion for dark mode: 
                    // - White/light backgrounds become pure black
                    // - Black text becomes white
                    // - Gray values are inverted for consistency
                    let inverted_r = 255 - r;
                    let inverted_g = 255 - g;
                    let inverted_b = 255 - b;
                    
                    black_bg_image.put_pixel(x, y, Rgba([inverted_r, inverted_g, inverted_b, a]));
                }
                
                let final_image = DynamicImage::ImageRgba8(black_bg_image);
                // eprintln!("[SYSTEM] ✅ Page rendered successfully: {}x{}", final_image.width(), final_image.height());
                return Ok(final_image);
            }
            return Err(anyhow::anyhow!("Output file not found at {:?}", output_file));
        }
        
        // eprintln!("[SYSTEM] Loading rendered page from {:?}", output_file);
        let image = image::open(&output_file)?;
        
        // Convert white backgrounds to pure black to match terminal
        use image::{Rgba, ImageBuffer};
        let rgba_image = image.to_rgba8();
        let (width, height) = rgba_image.dimensions();
        
        let mut black_bg_image = ImageBuffer::new(width, height);
        
        for (x, y, pixel) in rgba_image.enumerate_pixels() {
            let Rgba([r, g, b, a]) = *pixel;
            
            // Full inversion for dark mode: 
            // - White/light backgrounds become pure black
            // - Black text becomes white
            // - Gray values are inverted for consistency
            let inverted_r = 255 - r;
            let inverted_g = 255 - g;
            let inverted_b = 255 - b;
            
            black_bg_image.put_pixel(x, y, Rgba([inverted_r, inverted_g, inverted_b, a]));
        }
        
        let final_image = DynamicImage::ImageRgba8(black_bg_image);
        
        // Save a debug copy
        final_image.save("/tmp/system_render_output.png").ok();
        // eprintln!("[SYSTEM] ✅ Page rendered successfully: {}x{} - saved to /tmp/system_render_output.png", 
        //          final_image.width(), final_image.height());
        
        Ok(final_image)
    }
    
    /// Create an error image with a message when PDF rendering fails
    fn create_error_image(&self, width: u32, height: u32, message: &str) -> Result<DynamicImage> {
        use image::{Rgba, ImageBuffer, DynamicImage};
        
        // Create a black image with error message
        let width = width.max(200);
        let height = height.max(100);
        
        let mut img = ImageBuffer::from_pixel(width, height, Rgba([0u8, 0u8, 0u8, 255u8]));
        
        // Draw a simple border
        for x in 0..width {
            img.put_pixel(x, 0, Rgba([64, 64, 64, 255]));
            img.put_pixel(x, height - 1, Rgba([64, 64, 64, 255]));
        }
        for y in 0..height {
            img.put_pixel(0, y, Rgba([64, 64, 64, 255]));
            img.put_pixel(width - 1, y, Rgba([64, 64, 64, 255]));
        }
        
        // Add error text in center (simplified - just a colored rectangle for now)
        let text_width = message.len() as u32 * 8;
        let text_height = 16;
        let text_x = (width - text_width.min(width - 20)) / 2;
        let text_y = (height - text_height) / 2;
        
        // Draw a dark red background for the error message
        for y in text_y..(text_y + text_height).min(height) {
            for x in text_x..(text_x + text_width.min(width - 20)) {
                if x < width && y < height {
                    img.put_pixel(x, y, Rgba([64, 0, 0, 255]));
                }
            }
        }
        
        // Draw some placeholder "text" pixels (simplified rendering)
        // In a real implementation, you'd use a font rendering library
        for (i, _) in message.chars().enumerate().take(((width - 40) / 8) as usize) {
            let px = text_x + 4 + (i as u32 * 8);
            let py = text_y + 4;
            if px < width - 4 && py < height - 4 {
                // Draw a simple dot pattern to indicate text
                img.put_pixel(px, py, Rgba([255, 128, 128, 255]));
                img.put_pixel(px + 1, py, Rgba([255, 128, 128, 255]));
                img.put_pixel(px, py + 1, Rgba([255, 128, 128, 255]));
            }
        }
        
        Ok(DynamicImage::ImageRgba8(img))
    }
}