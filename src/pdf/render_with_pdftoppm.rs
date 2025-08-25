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
        eprintln!("[SYSTEM] Using pdftoppm to render page {} at {}x{}", page_num, width, height);
        
        // Create a temporary directory for output
        let temp_dir = TempDir::new()?;
        let output_prefix = temp_dir.path().join("page");
        
        // Use pdftoppm to convert PDF page to PNG
        // page_num is 0-based in our code but pdftoppm uses 1-based
        let page = page_num + 1;
        
        let output = Command::new("pdftoppm")
            .args(&[
                "-png",                    // PNG format
                "-f", &page.to_string(),   // First page
                "-l", &page.to_string(),   // Last page (same as first for single page)
                "-scale-to-x", &width.to_string(),   // Scale to width
                "-scale-to-y", &height.to_string(),  // Scale to height
                pdf_path.to_str().unwrap(),          // Input PDF
                output_prefix.to_str().unwrap(),     // Output prefix
            ])
            .output()?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("pdftoppm failed: {}", stderr));
        }
        
        // Find the generated PNG file
        // pdftoppm adds -1.png for single page output
        let output_file = temp_dir.path().join(format!("page-{}.png", page));
        
        if !output_file.exists() {
            // Try without page number suffix
            let alt_file = temp_dir.path().join("page-1.png");
            if alt_file.exists() {
                eprintln!("[SYSTEM] Loading rendered page from {:?}", alt_file);
                let image = image::open(&alt_file)?;
                eprintln!("[SYSTEM] ✅ Page rendered successfully: {}x{}", image.width(), image.height());
                return Ok(image);
            }
            return Err(anyhow::anyhow!("Output file not found at {:?}", output_file));
        }
        
        eprintln!("[SYSTEM] Loading rendered page from {:?}", output_file);
        let image = image::open(&output_file)?;
        
        // Save a debug copy
        image.save("/tmp/system_render_output.png").ok();
        eprintln!("[SYSTEM] ✅ Page rendered successfully: {}x{} - saved to /tmp/system_render_output.png", 
                 image.width(), image.height());
        
        Ok(image)
    }
}