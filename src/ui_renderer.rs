// Dynamic UI renderer that reads from hot-reloadable config
use crate::ui_config::UIConfig;
use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor, Attribute, SetAttribute},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, stdout, Write};
use std::path::PathBuf;
use image::DynamicImage;
use chonker8::integrated_file_picker::IntegratedFilePicker;
use chonker8::{pdf_renderer, viuer_display, content_extractor, ascii_display};

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Demo,
    FilePicker,
    PdfViewer,
}

pub struct UIRenderer {
    config: UIConfig,
    pdf_content: Vec<Vec<char>>,
    current_page: usize,
    total_pages: usize,
    scroll_offset: usize,
    cursor_x: usize,
    cursor_y: usize,
    current_screen: Screen,
    available_screens: Vec<Screen>,
    file_picker: Option<IntegratedFilePicker>,
    current_pdf_path: Option<PathBuf>,
    current_pdf_image: Option<DynamicImage>,
    dark_mode: bool,
}

impl UIRenderer {
    pub fn new(config: UIConfig) -> Self {
        // Initialize the file picker
        let file_picker = match IntegratedFilePicker::new() {
            Ok(picker) => Some(picker),
            Err(e) => {
                eprintln!("Warning: Failed to initialize file picker: {}", e);
                None
            }
        };
        
        Self {
            config,
            pdf_content: vec![vec![' '; 80]; 24], // Default empty content
            current_page: 1,
            total_pages: 1,
            scroll_offset: 0,
            cursor_x: 0,
            cursor_y: 0,
            current_screen: Screen::Demo,
            available_screens: vec![Screen::Demo, Screen::FilePicker, Screen::PdfViewer],
            file_picker,
            current_pdf_path: None,
            current_pdf_image: None,
            dark_mode: false,
        }
    }
    
    pub fn update_config(&mut self, config: UIConfig) {
        self.config = config;
    }
    
    pub fn set_pdf_content(&mut self, content: Vec<Vec<char>>) {
        self.pdf_content = content;
    }
    
    pub fn set_total_pages(&mut self, total: usize) {
        self.total_pages = total;
    }
    
    pub fn render(&mut self) -> Result<()> {
        match self.current_screen {
            Screen::Demo => self.render_demo_screen(),
            Screen::FilePicker => self.render_file_picker_screen(),
            Screen::PdfViewer => self.render_pdf_screen(),
        }
    }
    
    pub fn render_with_file_picker(&self, file_picker: &mut IntegratedFilePicker) -> Result<()> {
        match self.current_screen {
            Screen::Demo => self.render_demo_screen(),
            Screen::FilePicker => self.render_integrated_file_picker_screen(file_picker),
            Screen::PdfViewer => self.render_pdf_screen(),
        }
    }
    
    fn render_demo_screen(&self) -> Result<()> {
        // Get terminal size first
        let (width, height) = terminal::size()?;
        
        // Complete clear - both buffer and scrollback in viewport
        execute!(
            stdout(), 
            Clear(ClearType::All),
            Clear(ClearType::FromCursorDown),
            MoveTo(0, 0),
            Hide
        )?;
        
        // Fill screen with spaces to ensure clean slate
        for y in 0..height {
            execute!(stdout(), MoveTo(0, y), Print(" ".repeat(width as usize)))?;
        }
        execute!(stdout(), MoveTo(0, 0))?;
        
        // Render based on mode
        match self.config.mode.as_str() {
            "split" => self.render_split_mode(width, height)?,
            "full" => self.render_full_mode(width, height)?,
            "minimal" => self.render_minimal_mode(width, height)?,
            _ => self.render_split_mode(width, height)?,
        }
        
        // Render status bar if enabled
        if self.config.layout.status_bar {
            self.render_status_bar(width, height)?;
        }
        
        // Reset colors and ensure everything is flushed
        execute!(stdout(), ResetColor)?;
        stdout().flush()?;
        Ok(())
    }
    
    fn render_file_picker_screen(&mut self) -> Result<()> {
        // Use the integrated file picker if available
        let (width, height) = terminal::size()?;
        
        if let Some(file_picker) = &mut self.file_picker {
            // Render the actual integrated file picker
            file_picker.render(width, height)?;
        } else {
            // Fallback when file picker is not available
            execute!(
                stdout(),
                Clear(ClearType::All),
                MoveTo(0, 0),
                SetForegroundColor(crossterm::style::Color::Yellow),
                Print("⚠️ File picker not available - using fallback"),
                ResetColor,
                MoveTo(0, 2),
                Print("Tab: Next Screen • Esc: Exit")
            )?;
            stdout().flush()?;
        }
        
        Ok(())
    }
    
    fn render_integrated_file_picker_screen(&self, file_picker: &mut IntegratedFilePicker) -> Result<()> {
        let (width, height) = terminal::size()?;
        file_picker.render(width, height)?;
        Ok(())
    }
    
    fn render_pdf_screen(&self) -> Result<()> {
        // Chonker7-style split view: PDF image on left, text extraction on right
        let (width, height) = terminal::size()?;
        let split_x = width / 2;
        
        execute!(
            stdout(),
            Clear(ClearType::All),
            MoveTo(0, 0),
            Hide
        )?;
        
        // Render PDF image on left side if available
        if let Some(image) = &self.current_pdf_image {
            // Show PDF debug info in TUI
            execute!(
                stdout(),
                MoveTo(2, 2),
                SetForegroundColor(Color::Green),
                Print(&format!("PDF: {} pages | Image: {}x{}", 
                    self.total_pages, image.width(), image.height())),
                ResetColor
            )?;
            
            // Try to display the PDF image
            match viuer_display::display_pdf_image(
                image, 0, 4, split_x - 1, height - 6, self.dark_mode
            ) {
                Ok(_) => {
                    // Image displayed successfully
                }
                Err(_) => {
                    // Fall back to ASCII display
                    match ascii_display::display_pdf_as_ascii(
                        image, 2, 6, split_x - 4, height - 10
                    ) {
                        Ok(_) => {
                            execute!(
                                stdout(),
                                MoveTo(2, height - 4),
                                SetForegroundColor(Color::Yellow),
                                Print("ASCII mode (viuer failed)"),
                                ResetColor
                            )?;
                        }
                        Err(e) => {
                            execute!(
                                stdout(),
                                MoveTo(2, 6),
                                SetForegroundColor(Color::Red),
                                Print(&format!("Display error: {}", e)),
                                ResetColor
                            )?;
                        }
                    }
                }
            }
        } else {
            // Show placeholder if no PDF loaded
            execute!(
                stdout(),
                MoveTo(2, 4),
                SetForegroundColor(Color::Yellow),
                Print("📄 PDF - TEST Screen"),
                MoveTo(2, 6),
                SetForegroundColor(Color::White),
                Print("No PDF loaded. Use file picker to select a PDF."),
                MoveTo(2, 8),
                SetForegroundColor(Color::Cyan),
                Print("Press Tab to go to File Picker"),
                ResetColor
            )?;
        }
        
        // Render text extraction on right side
        self.render_text_extraction_panel(split_x, 0, width - split_x, height - 2)?;
        
        // Status bar
        let status_text = if let Some(path) = &self.current_pdf_path {
            format!("PDF: {} | Page: {}/{} | Tab: Cycle • Esc: Exit", 
                path.file_name().unwrap_or_default().to_string_lossy(),
                self.current_page, 
                self.total_pages)
        } else {
            "PDF - TEST Screen | Tab: Cycle • Esc: Exit".to_string()
        };
        
        execute!(
            stdout(),
            MoveTo(0, height - 1),
            SetBackgroundColor(Color::DarkBlue),
            SetForegroundColor(Color::White),
            Print(format!(" {:<width$} ", status_text, width = width as usize - 2)),
            ResetColor
        )?;
        
        stdout().flush()?;
        Ok(())
    }
    
    fn render_split_mode(&self, width: u16, height: u16) -> Result<()> {
        let split_pos = (width as f32 * self.config.panels.pdf.width_percent / 100.0) as u16;
        
        // Render left panel
        match self.config.layout.left_panel.as_str() {
            "pdf" => self.render_pdf_panel(0, 0, split_pos, height - 2)?,
            "text" => self.render_text_panel(0, 0, split_pos, height - 2)?,
            _ => {}
        }
        
        // Render divider
        let (_, _, _, _, _, v_line, _, _) = self.config.get_border_chars();
        execute!(stdout(), SetForegroundColor(self.config.get_highlight_color()))?;
        for y in 0..height - 2 {
            execute!(stdout(), MoveTo(split_pos, y), Print(v_line))?;
        }
        
        // Render right panel
        match self.config.layout.right_panel.as_str() {
            "text" => self.render_text_panel(split_pos + 1, 0, width - split_pos - 1, height - 2)?,
            "grid" => self.render_grid_panel(split_pos + 1, 0, width - split_pos - 1, height - 2)?,
            "debug" => self.render_debug_panel(split_pos + 1, 0, width - split_pos - 1, height - 2)?,
            _ => {}
        }
        
        Ok(())
    }
    
    fn render_full_mode(&self, width: u16, height: u16) -> Result<()> {
        // Full screen PDF view
        self.render_pdf_panel(0, 0, width, height - 2)?;
        Ok(())
    }
    
    fn render_minimal_mode(&self, width: u16, height: u16) -> Result<()> {
        // Just text, no borders
        self.render_text_content(0, 0, width, height - 1)?;
        Ok(())
    }
    
    fn render_pdf_panel(&self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        let (tl, tr, bl, br, h_line, v_line, _, _) = self.config.get_border_chars();
        
        // Draw border if not "none"
        if self.config.theme.border != "none" {
            execute!(stdout(), SetForegroundColor(self.config.get_highlight_color()))?;
            
            // Top border
            execute!(stdout(), MoveTo(x, y), Print(tl))?;
            for i in 1..width - 1 {
                execute!(stdout(), MoveTo(x + i, y), Print(h_line))?;
            }
            execute!(stdout(), MoveTo(x + width - 1, y), Print(tr))?;
            
            // Side borders
            for i in 1..height - 1 {
                execute!(stdout(), MoveTo(x, y + i), Print(v_line))?;
                execute!(stdout(), MoveTo(x + width - 1, y + i), Print(v_line))?;
            }
            
            // Bottom border
            execute!(stdout(), MoveTo(x, y + height - 1), Print(bl))?;
            for i in 1..width - 1 {
                execute!(stdout(), MoveTo(x + i, y + height - 1), Print(h_line))?;
            }
            execute!(stdout(), MoveTo(x + width - 1, y + height - 1), Print(br))?;
        }
        
        // Draw title
        let title = format!(" PDF Page {}/{} ", self.current_page, self.total_pages);
        execute!(
            stdout(),
            MoveTo(x + 2, y),
            SetForegroundColor(self.config.get_highlight_color()),
            Print(&title),
            SetForegroundColor(self.config.get_text_color())
        )?;
        
        // Draw content
        let content_x = if self.config.theme.border != "none" { x + 1 } else { x };
        let content_y = if self.config.theme.border != "none" { y + 1 } else { y };
        let content_width = if self.config.theme.border != "none" { width - 2 } else { width };
        let content_height = if self.config.theme.border != "none" { height - 2 } else { height };
        
        self.render_pdf_content(content_x, content_y, content_width, content_height)?;
        
        Ok(())
    }
    
    fn render_text_panel(&self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        let (tl, tr, bl, br, h_line, v_line, _, _) = self.config.get_border_chars();
        
        // Draw border if not "none"
        if self.config.theme.border != "none" {
            execute!(stdout(), SetForegroundColor(self.config.get_highlight_color()))?;
            
            // Top border
            execute!(stdout(), MoveTo(x, y), Print(tl))?;
            for i in 1..width - 1 {
                execute!(stdout(), MoveTo(x + i, y), Print(h_line))?;
            }
            execute!(stdout(), MoveTo(x + width - 1, y), Print(tr))?;
            
            // Side borders
            for i in 1..height - 1 {
                execute!(stdout(), MoveTo(x, y + i), Print(v_line))?;
                execute!(stdout(), MoveTo(x + width - 1, y + i), Print(v_line))?;
            }
            
            // Bottom border
            execute!(stdout(), MoveTo(x, y + height - 1), Print(bl))?;
            for i in 1..width - 1 {
                execute!(stdout(), MoveTo(x + i, y + height - 1), Print(h_line))?;
            }
            execute!(stdout(), MoveTo(x + width - 1, y + height - 1), Print(br))?;
        }
        
        // Draw title
        let title = " Extracted Text ";
        execute!(
            stdout(),
            MoveTo(x + 2, y),
            SetForegroundColor(self.config.get_highlight_color()),
            Print(title),
            SetForegroundColor(self.config.get_text_color())
        )?;
        
        // Draw content
        let content_x = if self.config.theme.border != "none" { x + 1 } else { x };
        let content_y = if self.config.theme.border != "none" { y + 1 } else { y };
        let content_width = if self.config.theme.border != "none" { width - 2 } else { width };
        let content_height = if self.config.theme.border != "none" { height - 2 } else { height };
        
        self.render_text_content(content_x, content_y, content_width, content_height)?;
        
        // Show cursor if enabled
        if self.config.panels.text.show_cursor {
            execute!(
                stdout(),
                MoveTo(content_x + self.cursor_x as u16, content_y + self.cursor_y as u16),
                Show
            )?;
        }
        
        Ok(())
    }
    
    fn render_grid_panel(&self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        // Render as character grid (useful for debugging)
        execute!(stdout(), SetForegroundColor(Color::Green))?;
        for row in 0..height.min(self.pdf_content.len() as u16) {
            for col in 0..width.min(80) {
                if (row as usize) < self.pdf_content.len() && (col as usize) < self.pdf_content[row as usize].len() {
                    let ch = self.pdf_content[row as usize][col as usize];
                    execute!(stdout(), MoveTo(x + col, y + row), Print(ch))?;
                }
            }
        }
        Ok(())
    }
    
    fn render_debug_panel(&self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        execute!(stdout(), SetForegroundColor(Color::Cyan))?;
        let debug_info = vec![
            format!("Mode: {}", self.config.mode),
            format!("Theme: {}", self.config.theme.highlight),
            format!("Border: {}", self.config.theme.border),
            format!("Page: {}/{}", self.current_page, self.total_pages),
            format!("Scroll: {}", self.scroll_offset),
            format!("Cursor: ({}, {})", self.cursor_x, self.cursor_y),
            format!("Wrap: {}", self.config.panels.text.wrap_text),
            "".to_string(),
            "Hot-reload active!".to_string(),
            "Edit ui.toml to change".to_string(),
        ];
        
        for (i, line) in debug_info.iter().enumerate() {
            if i < height as usize {
                execute!(stdout(), MoveTo(x, y + i as u16), Print(line))?;
            }
        }
        Ok(())
    }
    
    fn render_pdf_content(&self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        execute!(stdout(), SetForegroundColor(self.config.get_text_color()))?;
        
        // Generate page-specific content
        let page_content = self.get_page_content();
        
        for row in 0..height.min(page_content.len() as u16) {
            let content_row = (self.scroll_offset + row as usize).min(page_content.len() - 1);
            let mut line = String::new();
            
            for col in 0..width.min(page_content[content_row].len() as u16) {
                line.push(page_content[content_row][col as usize]);
            }
            
            execute!(stdout(), MoveTo(x, y + row), Print(&line))?;
        }
        
        Ok(())
    }
    
    fn get_page_content(&self) -> Vec<Vec<char>> {
        // Always use the main PDF content - no more page 2 demo
        self.pdf_content.clone()
    }
    
    fn render_text_content(&self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        execute!(stdout(), SetForegroundColor(self.config.get_text_color()))?;
        
        // Extract text from pdf_content
        let text: String = self.pdf_content
            .iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        
        let lines: Vec<String> = if self.config.panels.text.wrap_text {
            // Simple word wrapping
            text.split('\n').flat_map(|line| {
                line.chars()
                    .collect::<Vec<_>>()
                    .chunks(width as usize)
                    .map(|chunk| chunk.iter().collect::<String>())
                    .collect::<Vec<_>>()
            }).collect()
        } else {
            text.lines().map(|s| s.to_string()).collect()
        };
        
        for (i, line) in lines.iter().skip(self.scroll_offset).take(height as usize).enumerate() {
            let display_line = if self.config.panels.text.line_numbers {
                format!("{:4} {}", self.scroll_offset + i + 1, line)
            } else {
                line.to_string()
            };
            
            execute!(stdout(), MoveTo(x, y + i as u16), Print(&display_line))?;
        }
        
        Ok(())
    }
    
    fn render_status_bar(&self, width: u16, height: u16) -> Result<()> {
        let status_y = height - 1;
        
        // Clear status bar line with inverse video for visibility
        execute!(
            stdout(),
            MoveTo(0, status_y),
            crossterm::style::SetAttribute(crossterm::style::Attribute::Reverse),
            Print(" ".repeat(width as usize)),
            crossterm::style::SetAttribute(crossterm::style::Attribute::NoReverse)
        )?;
        
        // Left side: screen and mode info
        let left_status = format!(" [{}] {} Page {}/{} ", 
            self.get_screen_name(),
            self.config.mode.to_uppercase(),
            self.current_page,
            self.total_pages
        );
        execute!(stdout(), MoveTo(0, status_y), Print(&left_status))?;
        
        // Center: hints
        let center_status = "q:quit n:next p:prev m:mode w:wrap r:reload";
        let center_x = (width / 2) - (center_status.len() as u16 / 2);
        execute!(stdout(), MoveTo(center_x, status_y), Print(center_status))?;
        
        // Right side: position
        let right_status = format!(" {}:{} ", self.cursor_y + 1, self.cursor_x + 1);
        let right_x = width - right_status.len() as u16;
        execute!(stdout(), MoveTo(right_x, status_y), Print(&right_status))?;
        
        Ok(())
    }
    
    // Navigation methods
    pub fn next_page(&mut self) {
        if self.current_page < self.total_pages {
            self.current_page += 1;
        } else {
            self.current_page = 1; // Cycle back to first page
        }
        self.scroll_offset = 0;
    }
    
    pub fn prev_page(&mut self) {
        if self.current_page > 1 {
            self.current_page -= 1;
            self.scroll_offset = 0;
        }
    }
    
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }
    
    pub fn scroll_down(&mut self) {
        if self.scroll_offset < self.pdf_content.len().saturating_sub(10) {
            self.scroll_offset += 1;
        }
    }
    
    pub fn toggle_mode(&mut self) {
        self.config.mode = match self.config.mode.as_str() {
            "split" => "full".to_string(),
            "full" => "minimal".to_string(),
            "minimal" => "split".to_string(),
            _ => "split".to_string(),
        };
    }
    
    pub fn toggle_wrap(&mut self) {
        self.config.panels.text.wrap_text = !self.config.panels.text.wrap_text;
    }
    
    pub fn next_screen(&mut self) {
        let current_index = self.available_screens.iter()
            .position(|s| s == &self.current_screen)
            .unwrap_or(0);
        let next_index = (current_index + 1) % self.available_screens.len();
        self.current_screen = self.available_screens[next_index].clone();
    }
    
    pub fn prev_screen(&mut self) {
        let current_index = self.available_screens.iter()
            .position(|s| s == &self.current_screen)
            .unwrap_or(0);
        let prev_index = if current_index == 0 {
            self.available_screens.len() - 1
        } else {
            current_index - 1
        };
        self.current_screen = self.available_screens[prev_index].clone();
    }
    
    pub fn get_current_screen(&self) -> &Screen {
        &self.current_screen
    }
    
    pub fn current_screen(&self) -> &Screen {
        &self.current_screen
    }
    
    pub fn set_screen(&mut self, screen: Screen) {
        self.current_screen = screen;
    }
    
    pub fn handle_file_picker_input(&mut self, key: crossterm::event::KeyEvent) -> Result<Option<String>> {
        if let Some(file_picker) = &mut self.file_picker {
            match key.code {
                crossterm::event::KeyCode::Char(c) => {
                    file_picker.handle_char(c)?;
                }
                crossterm::event::KeyCode::Backspace => {
                    file_picker.handle_backspace()?;
                }
                crossterm::event::KeyCode::Up => {
                    file_picker.handle_up()?;
                }
                crossterm::event::KeyCode::Down => {
                    file_picker.handle_down()?;
                }
                crossterm::event::KeyCode::Enter => {
                    if let Some(selected_file) = file_picker.get_selected_file() {
                        return Ok(Some(selected_file.to_string_lossy().to_string()));
                    }
                }
                _ => {}
            }
        }
        Ok(None)
    }
    
    pub fn get_screen_name(&self) -> &str {
        match self.current_screen {
            Screen::Demo => "Demo",
            Screen::FilePicker => "File Picker", 
            Screen::PdfViewer => "PDF Viewer",
        }
    }
    
    pub fn load_pdf(&mut self, pdf_path: PathBuf) -> Result<()> {
        use crate::pdf_extraction::{DocumentAnalyzer};
        
        eprintln!("[DEBUG] UIRenderer::load_pdf called with: {:?}", pdf_path);
        
        // Load PDF page count - chonker7 style with fresh instance
        eprintln!("[DEBUG] Getting page count...");
        self.total_pages = content_extractor::get_page_count(&pdf_path)?;
        self.current_page = 1;
        eprintln!("[DEBUG] Page count: {}", self.total_pages);
        
        // Render first page image with larger dimensions for better visibility
        eprintln!("[DEBUG] Rendering PDF page...");
        let image = pdf_renderer::render_pdf_page(&pdf_path, 0, 1200, 1600)?;
        eprintln!("[DEBUG] PDF page rendered");
        
        // Use intelligent document-agnostic extraction
        eprintln!("[DEBUG] Creating analyzer...");
        let analyzer = DocumentAnalyzer::new()?;
        eprintln!("[DEBUG] Analyzing page...");
        let fingerprint = analyzer.analyze_page(&pdf_path, 0)?;
        eprintln!("[DEBUG] Analysis complete: text={:.1}%, image={:.1}%, has_tables={}, text_quality={:.2}", 
            fingerprint.text_coverage * 100.0, 
            fingerprint.image_coverage * 100.0,
            fingerprint.has_tables,
            fingerprint.text_quality);
        
        // Use the intelligent ExtractionRouter to extract text
        eprintln!("[DEBUG] Starting intelligent text extraction...");
        let extraction_result = crate::pdf_extraction::ExtractionRouter::extract_with_fallback_sync(&pdf_path, 0, &fingerprint)?;
        eprintln!("[DEBUG] Extraction complete using method: {:?}, quality: {:.2}", 
            extraction_result.method, extraction_result.quality_score);
        
        let text = extraction_result.text;
        
        // Convert extracted text to grid format for display
        let text_matrix = self.text_to_matrix(&text, 200, 100);
        
        // Update state
        self.current_pdf_path = Some(pdf_path);
        self.current_pdf_image = Some(image);
        self.pdf_content = text_matrix;
        
        // Store fingerprint info for display
        self.dark_mode = fingerprint.text_coverage > 0.8; // Just as a flag for now
        
        Ok(())
    }
    
    fn extract_text_simple(&self, pdf_path: &PathBuf, page: usize) -> Result<String> {
        use std::process::Command;
        
        // Try pdftotext first (cleaner output)
        let output = Command::new("pdftotext")
            .args(&[
                "-f", &(page + 1).to_string(),
                "-l", &(page + 1).to_string(),
                "-layout",
                pdf_path.to_str().unwrap(),
                "-"
            ])
            .output();
            
        if let Ok(output) = output {
            if output.status.success() {
                return Ok(String::from_utf8_lossy(&output.stdout).to_string());
            }
        }
        
        // Fallback to simple text
        Ok("PDF text extraction in progress...".to_string())
    }
    
    fn text_to_matrix(&self, text: &str, width: usize, height: usize) -> Vec<Vec<char>> {
        let mut matrix = vec![vec![' '; width]; height];
        let lines: Vec<&str> = text.lines().collect();
        
        for (y, line) in lines.iter().take(height).enumerate() {
            for (x, ch) in line.chars().take(width).enumerate() {
                matrix[y][x] = ch;
            }
        }
        
        matrix
    }
    
    pub fn get_current_pdf_path(&self) -> Option<&PathBuf> {
        self.current_pdf_path.as_ref()
    }
    
    fn render_text_extraction_panel(&self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        // Draw border
        execute!(stdout(), SetForegroundColor(Color::DarkGrey))?;
        for row in 0..height {
            execute!(stdout(), MoveTo(x, y + row), Print("│"))?; // Left border
        }
        
        // Title
        execute!(
            stdout(),
            MoveTo(x + 2, y + 1),
            SetForegroundColor(Color::Green),
            Print("Text Extraction"),
            ResetColor
        )?;
        
        // Render extracted text content
        let content_start_y = y + 3;
        let content_height = height.saturating_sub(4);
        let content_width = width.saturating_sub(4);
        
        for (row_idx, row) in self.pdf_content.iter().enumerate().take(content_height as usize) {
            let display_y = content_start_y + row_idx as u16;
            if display_y >= y + height {
                break;
            }
            
            execute!(stdout(), MoveTo(x + 2, display_y))?;
            
            // Convert chars to string for display
            let line: String = row.iter().take(content_width as usize).collect();
            execute!(
                stdout(),
                SetForegroundColor(Color::White),
                Print(&line),
                ResetColor
            )?;
        }
        
        Ok(())
    }
}