// Enhanced UI renderer with document-agnostic extraction display
use crate::ui_config::UIConfig;
use crate::pdf_extraction::{DocumentAnalyzer, ExtractionRouter, PageFingerprint};
use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{stdout, Write};
use std::path::PathBuf;
use image::DynamicImage;

pub struct EnhancedUIRenderer {
    config: UIConfig,
    current_page: usize,
    total_pages: usize,
    current_pdf_path: Option<PathBuf>,
    current_pdf_image: Option<DynamicImage>,
    
    // Extraction results
    page_fingerprint: Option<PageFingerprint>,
    extraction_method: Option<String>,
    extraction_quality: Option<f32>,
    extracted_text: String,
    extraction_time_ms: u64,
}

impl EnhancedUIRenderer {
    pub fn new(config: UIConfig) -> Self {
        Self {
            config,
            current_page: 1,
            total_pages: 1,
            current_pdf_path: None,
            current_pdf_image: None,
            page_fingerprint: None,
            extraction_method: None,
            extraction_quality: None,
            extracted_text: String::new(),
            extraction_time_ms: 0,
        }
    }
    
    pub async fn load_pdf(&mut self, pdf_path: &PathBuf, page: usize) -> Result<()> {
        self.current_pdf_path = Some(pdf_path.clone());
        self.current_page = page + 1;
        
        // Get total pages
        self.total_pages = chonker8::pdf_extraction::basic::get_page_count(pdf_path)?;
        
        // Load PDF image
        if let Ok(image) = chonker8::pdf_renderer::render_pdf_as_image(pdf_path, page) {
            self.current_pdf_image = Some(image);
        }
        
        // Perform document-agnostic extraction
        let analyzer = DocumentAnalyzer::new()?;
        let fingerprint = analyzer.analyze_page(pdf_path, page)?;
        
        let start = std::time::Instant::now();
        let extraction_result = ExtractionRouter::extract_with_fallback(
            pdf_path, 
            page, 
            &fingerprint
        ).await?;
        
        self.extraction_time_ms = start.elapsed().as_millis() as u64;
        self.page_fingerprint = Some(fingerprint);
        self.extraction_method = Some(format!("{:?}", extraction_result.method));
        self.extraction_quality = Some(extraction_result.quality_score);
        self.extracted_text = extraction_result.text;
        
        Ok(())
    }
    
    pub fn render(&self) -> Result<()> {
        let (width, height) = terminal::size()?;
        
        // Clear screen
        execute!(
            stdout(),
            Clear(ClearType::All),
            Hide,
            MoveTo(0, 0)
        )?;
        
        // Calculate split position (60% for PDF, 40% for extraction)
        let split_x = (width * 60 / 100).min(width - 40);
        
        // Render header
        self.render_header(width)?;
        
        // Render PDF image on left
        self.render_pdf_panel(0, 2, split_x, height - 3)?;
        
        // Render extraction results on right
        self.render_extraction_panel(split_x + 1, 2, width - split_x - 1, height - 3)?;
        
        // Render status bar
        self.render_status_bar(width, height)?;
        
        stdout().flush()?;
        Ok(())
    }
    
    fn render_header(&self, width: u16) -> Result<()> {
        execute!(
            stdout(),
            MoveTo(0, 0),
            SetBackgroundColor(Color::DarkBlue),
            SetForegroundColor(Color::Yellow),
            Print(format!(" {:^width$} ", "PDF PROCESSOR v4.0.0-HOTRELOAD", width = width as usize - 2)),
            ResetColor
        )?;
        Ok(())
    }
    
    fn render_pdf_panel(&self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        // Draw border
        execute!(
            stdout(),
            MoveTo(x, y),
            SetForegroundColor(Color::DarkGrey),
            Print("┌" ),
            Print("─".repeat((width - 2) as usize)),
            Print("┐")
        )?;
        
        for row in 1..height - 1 {
            execute!(
                stdout(),
                MoveTo(x, y + row),
                Print("│"),
                MoveTo(x + width - 1, y + row),
                Print("│")
            )?;
        }
        
        execute!(
            stdout(),
            MoveTo(x, y + height - 1),
            Print("└"),
            Print("─".repeat((width - 2) as usize)),
            Print("┘")
        )?;
        
        // PDF title
        execute!(
            stdout(),
            MoveTo(x + 2, y + 1),
            SetForegroundColor(Color::Cyan),
            Print(format!("📄 PDF IMAGE - Page {}/{}", self.current_page, self.total_pages)),
            ResetColor
        )?;
        
        // Display PDF image or placeholder
        if let Some(ref image) = self.current_pdf_image {
            // Use viuer to display image in terminal
            let display_config = viuer::Config {
                width: Some((width - 4) as u32),
                height: Some((height - 4) as u32),
                x: (x + 2) as u16,
                y: (y + 3) as i16,
                ..Default::default()
            };
            
            let _ = viuer::print(image, &display_config);
        } else {
            execute!(
                stdout(),
                MoveTo(x + 4, y + height / 2),
                SetForegroundColor(Color::Yellow),
                Print("No PDF loaded"),
                ResetColor
            )?;
        }
        
        Ok(())
    }
    
    fn render_extraction_panel(&self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        // Draw border
        execute!(
            stdout(),
            MoveTo(x, y),
            SetForegroundColor(Color::DarkGrey),
            Print("┌"),
            Print("─".repeat((width - 2) as usize)),
            Print("┐")
        )?;
        
        for row in 1..height - 1 {
            execute!(
                stdout(),
                MoveTo(x, y + row),
                Print("│"),
                MoveTo(x + width - 1, y + row),
                Print("│")
            )?;
        }
        
        execute!(
            stdout(),
            MoveTo(x, y + height - 1),
            Print("└"),
            Print("─".repeat((width - 2) as usize)),
            Print("┘")
        )?;
        
        // Title
        execute!(
            stdout(),
            MoveTo(x + 2, y + 1),
            SetForegroundColor(Color::Green),
            Print("🔍 INTELLIGENT EXTRACTION"),
            ResetColor
        )?;
        
        let mut current_y = y + 3;
        
        // Display fingerprint
        if let Some(ref fingerprint) = self.page_fingerprint {
            execute!(
                stdout(),
                MoveTo(x + 2, current_y),
                SetForegroundColor(Color::Yellow),
                Print("PAGE ANALYSIS:"),
                ResetColor
            )?;
            current_y += 1;
            
            execute!(
                stdout(),
                MoveTo(x + 4, current_y),
                Print(format!("• Text coverage: {:.1}%", fingerprint.text_coverage * 100.0))
            )?;
            current_y += 1;
            
            execute!(
                stdout(),
                MoveTo(x + 4, current_y),
                Print(format!("• Image coverage: {:.1}%", fingerprint.image_coverage * 100.0))
            )?;
            current_y += 1;
            
            execute!(
                stdout(),
                MoveTo(x + 4, current_y),
                Print(format!("• Text quality: {:.2}", fingerprint.text_quality))
            )?;
            current_y += 1;
            
            execute!(
                stdout(),
                MoveTo(x + 4, current_y),
                Print(format!("• Has tables: {}", fingerprint.has_tables))
            )?;
            current_y += 2;
        }
        
        // Display extraction method
        if let Some(ref method) = self.extraction_method {
            execute!(
                stdout(),
                MoveTo(x + 2, current_y),
                SetForegroundColor(Color::Cyan),
                Print(format!("METHOD: {}", method)),
                ResetColor
            )?;
            current_y += 1;
        }
        
        // Display quality score
        if let Some(quality) = self.extraction_quality {
            let color = if quality > 0.7 {
                Color::Green
            } else if quality > 0.4 {
                Color::Yellow
            } else {
                Color::Red
            };
            
            execute!(
                stdout(),
                MoveTo(x + 2, current_y),
                SetForegroundColor(color),
                Print(format!("QUALITY: {:.2} | TIME: {}ms", quality, self.extraction_time_ms)),
                ResetColor
            )?;
            current_y += 2;
        }
        
        // Display extracted text
        execute!(
            stdout(),
            MoveTo(x + 2, current_y),
            SetForegroundColor(Color::White),
            Print("EXTRACTED TEXT:"),
            ResetColor
        )?;
        current_y += 1;
        
        execute!(
            stdout(),
            MoveTo(x + 2, current_y),
            SetForegroundColor(Color::DarkGrey),
            Print("─".repeat((width - 4) as usize)),
            ResetColor
        )?;
        current_y += 1;
        
        // Display text content (wrapped and truncated)
        let text_lines: Vec<&str> = self.extracted_text.lines().collect();
        let max_lines = (height - (current_y - y) - 2) as usize;
        let text_width = (width - 4) as usize;
        
        for (i, line) in text_lines.iter().take(max_lines).enumerate() {
            let display_line = if line.len() > text_width {
                &line[..text_width]
            } else {
                line
            };
            
            execute!(
                stdout(),
                MoveTo(x + 2, current_y + i as u16),
                SetForegroundColor(Color::White),
                Print(display_line),
                ResetColor
            )?;
        }
        
        Ok(())
    }
    
    fn render_status_bar(&self, width: u16, height: u16) -> Result<()> {
        let status_text = if let Some(ref path) = self.current_pdf_path {
            format!(
                " {} | Page {}/{} | Method: {} | Quality: {:.2} ",
                path.file_name().unwrap_or_default().to_string_lossy(),
                self.current_page,
                self.total_pages,
                self.extraction_method.as_ref().unwrap_or(&"N/A".to_string()),
                self.extraction_quality.unwrap_or(0.0)
            )
        } else {
            " No PDF loaded | Press 'o' to open file ".to_string()
        };
        
        execute!(
            stdout(),
            MoveTo(0, height - 1),
            SetBackgroundColor(Color::DarkBlue),
            SetForegroundColor(Color::White),
            Print(format!("{:<width$}", status_text, width = width as usize)),
            ResetColor
        )?;
        
        Ok(())
    }
}