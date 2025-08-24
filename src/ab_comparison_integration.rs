// Integration module for A-B PDF comparison workflow
// Combines lopdf-vello-kitty rendering with pdftotext extraction

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;
use image::DynamicImage;
use crate::enhanced_ab_ui::EnhancedABComparison;
use crate::pdf_renderer;
use crate::kitty_protocol::KittyProtocol;

pub struct ABComparisonWorkflow {
    ui: EnhancedABComparison,
    kitty: KittyProtocol,
    current_pdf_path: Option<PathBuf>,
    current_page: usize,
    total_pages: usize,
    pdf_image_cache: Option<DynamicImage>,
    extraction_cache: Vec<Vec<char>>,
    image_id: Option<u32>,
}

impl ABComparisonWorkflow {
    pub fn new() -> Result<Self> {
        let mut kitty = KittyProtocol::new();
        
        // Force enable Kitty for best quality
        if !kitty.is_supported() {
            eprintln!("[INFO] Kitty protocol not detected, forcing enable for best PDF rendering");
            kitty.force_enable();
        }
        
        Ok(Self {
            ui: EnhancedABComparison::new(),
            kitty,
            current_pdf_path: None,
            current_page: 1,
            total_pages: 1,
            pdf_image_cache: None,
            extraction_cache: Vec::new(),
            image_id: None,
        })
    }
    
    /// Load a PDF for comparison
    pub fn load_pdf(&mut self, path: PathBuf) -> Result<()> {
        eprintln!("[DEBUG] Loading PDF for A-B comparison: {:?}", path);
        
        // Get page count
        self.total_pages = pdf_renderer::get_pdf_page_count(&path)?;
        self.current_pdf_path = Some(path.clone());
        self.current_page = 1;
        
        // Load first page
        self.load_current_page()?;
        
        Ok(())
    }
    
    /// Load current page for both panels
    fn load_current_page(&mut self) -> Result<()> {
        if let Some(ref path) = self.current_pdf_path {
            // 1. Render PDF with Vello (left panel)
            let pdf_image = self.render_pdf_page(path)?;
            
            // 2. Extract text with pdftotext (right panel)
            let extraction = self.extract_with_pdftotext(path)?;
            
            // 3. Update UI with both
            self.ui.load_pdf_content(pdf_image.clone(), extraction.clone());
            
            // Cache for performance
            self.pdf_image_cache = Some(pdf_image);
            self.extraction_cache = extraction;
        }
        
        Ok(())
    }
    
    /// Render PDF page using Vello GPU acceleration
    fn render_pdf_page(&self, path: &PathBuf) -> Result<DynamicImage> {
        eprintln!("[DEBUG] Rendering page {} with Vello", self.current_page);
        
        // High resolution for quality
        let image = pdf_renderer::render_pdf_page(
            path,
            self.current_page - 1,
            1600,  // Width
            2200   // Height
        )?;
        
        Ok(image)
    }
    
    /// Extract text using pdftotext with layout preservation
    fn extract_with_pdftotext(&self, path: &PathBuf) -> Result<Vec<Vec<char>>> {
        eprintln!("[DEBUG] Extracting page {} with pdftotext", self.current_page);
        
        let output = Command::new("pdftotext")
            .args(&[
                "-f", &self.current_page.to_string(),
                "-l", &self.current_page.to_string(),
                "-layout",  // Preserve spatial layout
                "-nopgbrk", // No page breaks
                path.to_str().unwrap(),
                "-"
            ])
            .output()?;
        
        if !output.status.success() {
            anyhow::bail!("pdftotext extraction failed");
        }
        
        let text = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = text.lines().collect();
        
        // Convert to character grid
        let max_width = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let mut grid = vec![vec![' '; max_width]; lines.len()];
        
        for (row, line) in lines.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                if col < max_width {
                    grid[row][col] = ch;
                }
            }
        }
        
        Ok(grid)
    }
    
    /// Display PDF image in left panel using Kitty protocol
    pub fn display_pdf_image(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        if let Some(ref image) = self.pdf_image_cache {
            // Clear previous image
            if let Some(id) = self.image_id {
                let _ = self.kitty.clear_image(id);
            }
            
            // Calculate display dimensions
            let cell_width = 9;
            let cell_height = 18;
            let display_width = (width as u32) * cell_width;
            let display_height = (height as u32) * cell_height;
            
            // Scale to fit
            let scale_x = display_width as f32 / image.width() as f32;
            let scale_y = display_height as f32 / image.height() as f32;
            let scale = scale_x.min(scale_y);
            
            let final_width = (image.width() as f32 * scale) as u32;
            let final_height = (image.height() as f32 * scale) as u32;
            
            // Display with Kitty
            match self.kitty.display_image(
                image,
                x as u32,
                y as u32,
                Some(final_width),
                Some(final_height),
            ) {
                Ok(id) => {
                    self.image_id = Some(id);
                    eprintln!("[DEBUG] PDF image displayed with ID: {}", id);
                }
                Err(e) => {
                    eprintln!("[ERROR] Failed to display PDF image: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Render the full A-B comparison view
    pub fn render(&mut self) -> Result<()> {
        // Render UI
        self.ui.render_split_view()?;
        
        // Display PDF image in left panel
        let (width, height) = crossterm::terminal::size()?;
        let split_x = width / 2;
        self.display_pdf_image(1, 1, split_x - 2, height - 3)?;
        
        Ok(())
    }
    
    /// Navigation methods
    pub fn next_page(&mut self) -> Result<()> {
        if self.current_page < self.total_pages {
            self.current_page += 1;
            self.load_current_page()?;
        }
        Ok(())
    }
    
    pub fn prev_page(&mut self) -> Result<()> {
        if self.current_page > 1 {
            self.current_page -= 1;
            self.load_current_page()?;
        }
        Ok(())
    }
    
    /// Scroll handling
    pub fn scroll_up(&mut self, lines: u16) {
        self.ui.scroll(-(lines as i16));
    }
    
    pub fn scroll_down(&mut self, lines: u16) {
        self.ui.scroll(lines as i16);
    }
    
    /// Mode toggles
    pub fn toggle_edit_mode(&mut self) {
        self.ui.toggle_edit_mode();
    }
    
    pub fn toggle_sync_scroll(&mut self) {
        self.ui.toggle_sync_scroll();
    }
    
    /// Save corrected extraction
    pub fn save_corrections(&self) -> Result<()> {
        if let Some(ref pdf_path) = self.current_pdf_path {
            let txt_path = pdf_path.with_extension("corrected.txt");
            self.ui.save_corrections(&txt_path)?;
            eprintln!("[INFO] Saved corrections to {:?}", txt_path);
        }
        Ok(())
    }
    
    /// Future: Add vision model annotations
    pub fn add_vision_suggestion(&mut self, line: usize, suggestion: String) {
        self.ui.add_vision_annotation(line, suggestion);
    }
    
    /// Get extraction quality metrics
    pub fn get_extraction_metrics(&self) -> ExtractionMetrics {
        let total_chars = self.extraction_cache.iter()
            .map(|row| row.iter().filter(|&&c| c != ' ').count())
            .sum::<usize>();
        
        let total_lines = self.extraction_cache.len();
        
        ExtractionMetrics {
            total_characters: total_chars,
            total_lines,
            page_number: self.current_page,
            extraction_method: "pdftotext".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct ExtractionMetrics {
    pub total_characters: usize,
    pub total_lines: usize,
    pub page_number: usize,
    pub extraction_method: String,
}