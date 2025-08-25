// Enhanced A-B Comparison UI for chonker8-hot
// Perfect 50/50 split with PDF image (left) and pdftotext extraction (right)
// Dark mode optimized for visual comparison and editing

use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{stdout, Write};
use std::path::PathBuf;
use image::{DynamicImage, ImageBuffer, Rgba};
use crate::kitty_protocol::KittyProtocol;

// Dark theme colors optimized for PDF comparison
pub struct DarkTheme {
    pub bg_primary: Color,      // Main background
    pub bg_secondary: Color,     // Panel backgrounds
    pub border: Color,           // Dividers and borders
    pub text_primary: Color,     // Main text
    pub text_secondary: Color,   // Labels and headers
    pub highlight: Color,        // Selected/active items
    pub success: Color,          // Good extraction
    pub warning: Color,          // Medium confidence
    pub error: Color,            // Low confidence
}

impl Default for DarkTheme {
    fn default() -> Self {
        Self {
            bg_primary: Color::Rgb { r: 15, g: 15, b: 20 },
            bg_secondary: Color::Rgb { r: 25, g: 25, b: 35 },
            border: Color::Rgb { r: 50, g: 50, b: 70 },
            text_primary: Color::Rgb { r: 220, g: 220, b: 230 },
            text_secondary: Color::Rgb { r: 150, g: 150, b: 170 },
            highlight: Color::Rgb { r: 100, g: 150, b: 255 },
            success: Color::Rgb { r: 100, g: 255, b: 150 },
            warning: Color::Rgb { r: 255, g: 200, b: 100 },
            error: Color::Rgb { r: 255, g: 100, b: 100 },
        }
    }
}

pub struct EnhancedABComparison {
    theme: DarkTheme,
    pdf_image: Option<DynamicImage>,
    extracted_text: Vec<String>,
    extraction_layout: Vec<Vec<char>>, // Raw pdftotext layout
    current_page: usize,
    total_pages: usize,
    pdf_scroll: usize,
    text_scroll: usize,
    sync_scroll: bool,
    edit_mode: bool,
    cursor_row: usize,
    cursor_col: usize,
    modified_lines: Vec<usize>,
    vision_annotations: Vec<(usize, String)>, // Future: vision model suggestions
    kitty_protocol: KittyProtocol,
    pdf_image_id: Option<u32>,  // Track Kitty image ID for updates
}

impl EnhancedABComparison {
    pub fn new() -> Self {
        Self {
            theme: DarkTheme::default(),
            pdf_image: None,
            extracted_text: Vec::new(),
            extraction_layout: Vec::new(),
            current_page: 1,
            total_pages: 1,
            pdf_scroll: 0,
            text_scroll: 0,
            sync_scroll: true,
            edit_mode: false,
            cursor_row: 0,
            cursor_col: 0,
            modified_lines: Vec::new(),
            vision_annotations: Vec::new(),
            kitty_protocol: KittyProtocol::new(),
            pdf_image_id: None,
        }
    }
    
    /// Apply dark mode enhancement to PDF image for better visibility
    pub fn enhance_pdf_for_dark_mode(&self, image: DynamicImage) -> DynamicImage {
        let rgba_image = image.to_rgba8();
        let (width, height) = rgba_image.dimensions();
        let mut buffer = ImageBuffer::new(width, height);
        
        for y in 0..height {
            for x in 0..width {
                let pixel = rgba_image.get_pixel(x, y);
                let Rgba([r, g, b, a]) = *pixel;
            
            // Calculate luminance
            let lum = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as u8;
            
            // Smart contrast enhancement for dark mode
            let (new_r, new_g, new_b) = if lum > 240 {
                // White background -> dark background
                (30, 30, 40)
            } else if lum > 200 {
                // Light gray -> darker
                (50, 50, 60)
            } else if lum < 50 {
                // Black text -> light text
                (220, 220, 230)
            } else {
                // Enhance mid-tones for better contrast
                let factor = if lum < 128 { 1.8 } else { 0.7 };
                (
                    (r as f32 * factor).min(255.0) as u8,
                    (g as f32 * factor).min(255.0) as u8,
                    (b as f32 * factor).min(255.0) as u8,
                )
            };
            
                buffer.put_pixel(x, y, Rgba([new_r, new_g, new_b, a]));
            }
        }
        
        DynamicImage::ImageRgba8(buffer)
    }
    
    /// Render the perfect 50/50 split comparison view
    pub fn render_split_view(&mut self) -> Result<()> {
        let (width, height) = terminal::size()?;
        
        // Clear with dark background
        execute!(
            stdout(),
            SetBackgroundColor(self.theme.bg_primary),
            Clear(ClearType::All),
            MoveTo(0, 0),
            Hide
        )?;
        
        // Calculate exact 50% split
        let split_x = width / 2;
        
        // Render PDF panel (left)
        self.render_pdf_panel(0, 0, split_x, height - 2)?;
        
        // Render divider with subtle styling
        self.render_divider(split_x, height - 2)?;
        
        // Render text panel (right)
        self.render_text_panel(split_x + 1, 0, width - split_x - 1, height - 2)?;
        
        // Render status bar
        self.render_status_bar(width, height)?;
        
        stdout().flush()?;
        Ok(())
    }
    
    fn render_pdf_panel(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        // Panel header
        execute!(
            stdout(),
            MoveTo(x, y),
            SetBackgroundColor(self.theme.bg_secondary),
            SetForegroundColor(self.theme.text_secondary),
            Print(format!("┌─ 📄 PDF Page {}/{} ", self.current_page, self.total_pages)),
            Print("─".repeat((width as usize).saturating_sub(20))),
            Print("┐")
        )?;
        
        // Clear PDF content area
        for row in 1..height {
            execute!(
                stdout(),
                MoveTo(x, y + row),
                SetBackgroundColor(self.theme.bg_secondary),
                Print(" ".repeat(width as usize))
            )?;
        }
        
        // Display PDF image using Kitty protocol
        if let Some(ref pdf_image) = self.pdf_image {
            // Calculate position and size for the image
            // Leave 1 row for header, 1 for footer
            let img_x = x + 1;
            let img_y = y + 1;
            let img_width = width.saturating_sub(2);
            let img_height = height.saturating_sub(2);
            
            // Display the image via Kitty protocol
            match self.kitty_protocol.display_image(
                pdf_image, 
                img_x as u32, 
                img_y as u32,
                Some(img_width as u32),
                Some(img_height as u32)
            ) {
                Ok(image_id) => {
                    self.pdf_image_id = Some(image_id);
                    eprintln!("[UI] Displayed PDF via Kitty protocol with ID {}", image_id);
                }
                Err(e) => {
                    // Fallback text if Kitty not supported
                    execute!(
                        stdout(),
                        MoveTo(x + 2, y + height / 2),
                        SetForegroundColor(self.theme.text_secondary),
                        Print(format!("[PDF Image Ready - Kitty Protocol: {}]", e))
                    )?;
                }
            }
        } else {
            execute!(
                stdout(),
                MoveTo(x + 2, y + height / 2),
                SetForegroundColor(self.theme.warning),
                Print("No PDF loaded")
            )?;
        }
        
        Ok(())
    }
    
    fn render_text_panel(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        // Panel header with mode indicator
        let header = if self.edit_mode {
            format!("┌─ ✏️  EDIT MODE - pdftotext extraction ")
        } else {
            format!("┌─ 👁  VIEW MODE - pdftotext extraction ")
        };
        
        execute!(
            stdout(),
            MoveTo(x, y),
            SetBackgroundColor(self.theme.bg_secondary),
            SetForegroundColor(self.theme.highlight),
            Print(&header),
            SetForegroundColor(self.theme.text_secondary),
            Print("─".repeat((width as usize).saturating_sub(header.len() + 1))),
            Print("┐")
        )?;
        
        // Render extracted text with layout preservation
        let start_line = if self.sync_scroll { self.pdf_scroll } else { self.text_scroll };
        
        for row in 0..(height - 1).min(self.extraction_layout.len() as u16) {
            let line_idx = start_line + row as usize;
            
            if line_idx < self.extraction_layout.len() {
                execute!(stdout(), MoveTo(x, y + row + 1))?;
                
                // Line number
                execute!(
                    stdout(),
                    SetForegroundColor(self.theme.text_secondary),
                    Print(format!("{:4} │ ", line_idx + 1))
                )?;
                
                // Text content with modification highlighting
                let line_color = if self.modified_lines.contains(&line_idx) {
                    self.theme.success // Green for edited lines
                } else if self.has_vision_annotation(line_idx) {
                    self.theme.warning // Yellow for vision suggestions
                } else {
                    self.theme.text_primary // Normal text
                };
                
                execute!(
                    stdout(),
                    SetForegroundColor(line_color)
                )?;
                
                // Render characters from layout grid
                if line_idx < self.extraction_layout.len() {
                    let line = &self.extraction_layout[line_idx];
                    let max_chars = (width as usize).saturating_sub(7); // Account for line numbers
                    
                    for (col, &ch) in line.iter().take(max_chars).enumerate() {
                        // Highlight cursor position in edit mode
                        if self.edit_mode && line_idx == self.cursor_row && col == self.cursor_col {
                            execute!(
                                stdout(),
                                SetBackgroundColor(self.theme.highlight),
                                Print(ch),
                                SetBackgroundColor(self.theme.bg_secondary)
                            )?;
                        } else {
                            execute!(stdout(), Print(ch))?;
                        }
                    }
                }
            }
        }
        
        // Show cursor in edit mode
        if self.edit_mode {
            execute!(stdout(), Show)?;
        }
        
        Ok(())
    }
    
    fn render_divider(&self, x: u16, height: u16) -> Result<()> {
        execute!(stdout(), SetForegroundColor(self.theme.border))?;
        
        for y in 0..height {
            execute!(
                stdout(),
                MoveTo(x, y),
                Print("│")
            )?;
        }
        
        Ok(())
    }
    
    fn render_status_bar(&self, width: u16, height: u16) -> Result<()> {
        let mode_indicator = if self.edit_mode { "EDIT" } else { "VIEW" };
        let sync_indicator = if self.sync_scroll { "SYNC" } else { "INDEPENDENT" };
        
        let left_status = format!(
            " Page {}/{} │ {} │ {} │ {} modified",
            self.current_page,
            self.total_pages,
            mode_indicator,
            sync_indicator,
            self.modified_lines.len()
        );
        
        let right_status = " ↑↓:Scroll │ e:Edit │ s:Sync │ →←:Pages │ w:Save │ q:Quit ";
        
        let padding = width as usize - left_status.len() - right_status.len();
        let status = format!(
            "{}{}{}",
            left_status,
            " ".repeat(padding.max(0)),
            right_status
        );
        
        execute!(
            stdout(),
            MoveTo(0, height - 1),
            SetBackgroundColor(self.theme.bg_secondary),
            SetForegroundColor(self.theme.text_secondary),
            Print(format!("{:width$}", status, width = width as usize)),
            ResetColor
        )?;
        
        Ok(())
    }
    
    fn has_vision_annotation(&self, line: usize) -> bool {
        self.vision_annotations.iter().any(|(l, _)| *l == line)
    }
    
    // Public interface methods
    pub fn load_pdf_content(&mut self, image: DynamicImage, layout: Vec<Vec<char>>) {
        self.pdf_image = Some(self.enhance_pdf_for_dark_mode(image));
        self.extraction_layout = layout;
        self.extracted_text = self.extraction_layout
            .iter()
            .map(|row| row.iter().collect::<String>().trim_end().to_string())
            .collect();
    }
    
    
    pub fn toggle_edit_mode(&mut self) {
        self.edit_mode = !self.edit_mode;
    }
    
    pub fn scroll_up(&mut self) {
        if self.sync_scroll {
            self.pdf_scroll = self.pdf_scroll.saturating_sub(1);
            self.text_scroll = self.text_scroll.saturating_sub(1);
        } else if self.edit_mode {
            self.text_scroll = self.text_scroll.saturating_sub(1);
        } else {
            self.pdf_scroll = self.pdf_scroll.saturating_sub(1);
        }
    }
    
    pub fn scroll_down(&mut self) {
        if self.sync_scroll {
            self.pdf_scroll += 1;
            self.text_scroll += 1;
        } else if self.edit_mode {
            self.text_scroll += 1;
        } else {
            self.pdf_scroll += 1;
        }
    }
    
    pub fn cleanup(&mut self) -> Result<()> {
        // Clear any Kitty images
        if let Some(image_id) = self.pdf_image_id {
            self.kitty_protocol.clear_image(image_id)?;
        }
        
        // Show cursor again
        use crossterm::cursor::Show;
        execute!(stdout(), Show)?;
        
        Ok(())
    }
    
    pub fn toggle_sync_scroll(&mut self) {
        self.sync_scroll = !self.sync_scroll;
        if self.sync_scroll {
            self.text_scroll = self.pdf_scroll;
        }
    }
    
    pub fn scroll(&mut self, delta: i16) {
        let max_scroll = self.extraction_layout.len().saturating_sub(10);
        
        if self.sync_scroll {
            self.pdf_scroll = (self.pdf_scroll as i16 + delta).max(0) as usize;
            self.pdf_scroll = self.pdf_scroll.min(max_scroll);
            self.text_scroll = self.pdf_scroll;
        } else {
            self.text_scroll = (self.text_scroll as i16 + delta).max(0) as usize;
            self.text_scroll = self.text_scroll.min(max_scroll);
        }
    }
    
    pub fn mark_line_modified(&mut self, line: usize) {
        if !self.modified_lines.contains(&line) {
            self.modified_lines.push(line);
        }
    }
    
    pub fn add_vision_annotation(&mut self, line: usize, suggestion: String) {
        self.vision_annotations.push((line, suggestion));
    }
    
    pub fn get_extracted_text(&self) -> String {
        self.extracted_text.join("\n")
    }
    
    pub fn save_corrections(&self, path: &PathBuf) -> Result<()> {
        let content = self.extraction_layout
            .iter()
            .map(|row| row.iter().collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        
        std::fs::write(path, content)?;
        Ok(())
    }
}