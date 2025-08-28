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
        
        // Use timeout command to prevent hanging on large files
        let output = match Command::new("timeout")
            .args(&[
                "15",                      // 15 second timeout for large PDFs
                "pdftoppm",
                "-png",                    // PNG format
                "-f", &page.to_string(),   // First page
                "-l", &page.to_string(),   // Last page (same as first for single page)
                "-scale-to-x", &width.to_string(),   // Scale to width
                "-scale-to-y", &height.to_string(),  // Scale to height
                pdf_path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid PDF path"))?,     // Input PDF
                output_prefix.to_str().ok_or_else(|| anyhow::anyhow!("Invalid output path"))?,     // Output prefix
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
            // Check if it was a timeout
            if output.status.code() == Some(124) {  // timeout command returns 124 on timeout
                eprintln!("⏱️ PDF rendering timed out after 15 seconds (file may be too large)");
                return self.create_error_image(width, height, "Timeout: PDF too large");
            }
            // Try to provide a helpful error message
            if stderr.contains("can't open file") {
                return self.create_error_image(width, height, "Cannot open PDF");
            } else if stderr.contains("Incorrect page") {
                return self.create_error_image(width, height, &format!("Page {} not found", page));
            } else {
                eprintln!("pdftoppm stderr: {}", stderr);
                return self.create_error_image(width, height, "PDF render failed");
            }
        }
        
        // Find the generated PNG file
        // pdftoppm uses different padding based on total page count:
        // - 1-99 pages: page-01.png, page-02.png (2 digits)
        // - 100-999 pages: page-001.png, page-002.png (3 digits)  
        // - 1000+ pages: page-0001.png, page-0002.png (4 digits)
        
        // Try different naming patterns in order of most common
        let possible_files = vec![
            temp_dir.path().join(format!("page-{:02}.png", page)),   // 2-digit (most common)
            temp_dir.path().join(format!("page-{:03}.png", page)),   // 3-digit
            temp_dir.path().join(format!("page-{:04}.png", page)),   // 4-digit
            temp_dir.path().join(format!("page-{}.png", page)),      // no padding
        ];
        
        let output_file = possible_files.iter()
            .find(|f| f.exists())
            .cloned();
            
        if let Some(output_file) = output_file {
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
        } else {
            // Debug: list what files were actually created
            let mut files_found = Vec::new();
            if let Ok(entries) = std::fs::read_dir(temp_dir.path()) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        files_found.push(entry.file_name().to_string_lossy().to_string());
                    }
                }
            }
            
            return Err(anyhow::anyhow!(
                "pdftoppm output file not found.\nTried patterns: page-{:02}.png, page-{:03}.png, page-{:04}.png, page-{}.png\nFiles created: {:?}", 
                page, page, page, page,
                files_found
            ));
        }
    }
    
    /// Create an error image with a message when PDF rendering fails
    fn create_error_image(&self, width: u32, height: u32, message: &str) -> Result<DynamicImage> {
        use image::{Rgba, ImageBuffer, DynamicImage};
        
        // Create a black image with error message
        let width = width.max(200);
        let height = height.max(100);
        
        let mut img = ImageBuffer::from_pixel(width, height, Rgba([0u8, 0u8, 0u8, 255u8]));
        
        // Draw a simple border
        // Fix: Use saturating arithmetic to prevent underflow
        for x in 0..width {
            img.put_pixel(x, 0, Rgba([64, 64, 64, 255]));
            if height > 0 {
                img.put_pixel(x, height.saturating_sub(1), Rgba([64, 64, 64, 255]));
            }
        }
        for y in 0..height {
            img.put_pixel(0, y, Rgba([64, 64, 64, 255]));
            if width > 0 {
                img.put_pixel(width.saturating_sub(1), y, Rgba([64, 64, 64, 255]));
            }
        }
        
        // Add error text in center (simplified - just a colored rectangle for now)
        // Fix: Use saturating arithmetic to prevent underflow
        let text_width = (message.len() as u32).saturating_mul(8);
        let text_height = 16;
        let text_x = width.saturating_sub(text_width.min(width.saturating_sub(20))) / 2;
        let text_y = height.saturating_sub(text_height) / 2;
        
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
        // Fix: Use saturating arithmetic to prevent underflow
        let max_chars = width.saturating_sub(40) / 8;
        for (i, _) in message.chars().enumerate().take(max_chars as usize) {
            let px = text_x.saturating_add(4).saturating_add(i as u32 * 8);
            let py = text_y.saturating_add(4);
            if px < width.saturating_sub(4) && py < height.saturating_sub(4) {
                // Draw a simple dot pattern to indicate text
                img.put_pixel(px, py, Rgba([255, 128, 128, 255]));
                img.put_pixel(px + 1, py, Rgba([255, 128, 128, 255]));
                img.put_pixel(px, py + 1, Rgba([255, 128, 128, 255]));
            }
        }
        
        Ok(DynamicImage::ImageRgba8(img))
    }
}