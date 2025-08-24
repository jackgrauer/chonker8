// Dynamic UI renderer that reads from hot-reloadable config
use crate::ui_config::UIConfig;
use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, stdout, Write};
use std::path::PathBuf;
use image::DynamicImage;
use chonker8::integrated_file_picker::IntegratedFilePicker;
use chonker8::{pdf_renderer, content_extractor};
use chonker8::kitty_protocol::KittyProtocol;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Demo,
    FilePicker,
    PdfViewer,
    Debug,
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
    extraction_method: Option<String>,
    extraction_quality: Option<f32>,
    extraction_timestamp: Option<String>,
    debug_messages: Vec<String>,
    debug_scroll_offset: usize,
    debug_messages_loaded: bool,
    kitty: KittyProtocol,
    current_image_id: Option<u32>,
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
        
        let mut kitty = KittyProtocol::new();
        
        // Log Kitty support status
        if kitty.is_supported() {
            eprintln!("[DEBUG] Kitty graphics protocol detected and enabled");
        } else {
            eprintln!("[DEBUG] Kitty graphics protocol not detected, using fallback rendering");
        }
        
        Self {
            config,
            pdf_content: vec![vec![' '; 80]; 24], // Default empty content
            current_page: 1,
            total_pages: 1,
            scroll_offset: 0,
            cursor_x: 0,
            cursor_y: 0,
            current_screen: Screen::Demo,
            available_screens: vec![Screen::Demo, Screen::FilePicker, Screen::PdfViewer, Screen::Debug],
            file_picker,
            current_pdf_path: None,
            current_pdf_image: None,
            dark_mode: false,
            extraction_method: None,
            extraction_quality: None,
            extraction_timestamp: None,
            debug_messages: Vec::new(),
            debug_scroll_offset: 0,
            debug_messages_loaded: false,
            kitty,
            current_image_id: None,
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
    
    pub fn add_debug_message(&mut self, message: String) {
        // Add timestamp to each message
        let timestamped = format!("[{}] {}", 
            chrono::Local::now().format("%H:%M:%S%.3f"), 
            message
        );
        self.debug_messages.push(timestamped.clone());
        
        // Also write to debug log file so it persists and can be loaded in DEBUG screen
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/chonker8_debug.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "[{}] [RUNTIME] {}", 
                chrono::Local::now().format("%H:%M:%S%.3f"), 
                message);
        }
        
        // Keep only last 1000 messages to avoid memory issues
        if self.debug_messages.len() > 1000 {
            self.debug_messages.drain(0..100);
        }
    }
    
    pub fn load_debug_log(&mut self) {
        // Read any new messages from the debug log file
        if let Ok(contents) = std::fs::read_to_string("/tmp/chonker8_debug.log") {
            for line in contents.lines() {
                // Check if we already have this message (avoid duplicates)
                if !self.debug_messages.contains(&line.to_string()) {
                    self.debug_messages.push(line.to_string());
                }
            }
            
            // Keep only last 1000 messages
            if self.debug_messages.len() > 1000 {
                self.debug_messages.drain(0..self.debug_messages.len() - 1000);
            }
            
            // Don't clear the log file - let it accumulate and rely on deduplication
            // This ensures build warnings persist across multiple reads
        }
    }
    
    fn get_message_color(&self, message: &str) -> Color {
        // Simple syntax highlighting based on message content
        if message.contains("ERROR") || message.contains("failed") || message.contains("error:") {
            Color::Red
        } else if message.contains("WARNING") || message.contains("warning:") {
            Color::Yellow
        } else if message.contains("SUCCESS") || message.contains("successful") || message.contains("complete") {
            Color::Green
        } else if message.contains("[EXTRACTION]") || message.contains("[RUNTIME]") {
            Color::Cyan
        } else if message.contains("[BUILD]") {
            Color::Blue
        } else {
            Color::White
        }
    }
    
    pub fn render(&mut self) -> Result<()> {
        eprintln!("[DEBUG] Rendering screen: {:?}", self.current_screen);
        match self.current_screen {
            Screen::Demo => self.render_demo_screen(),
            Screen::FilePicker => self.render_file_picker_screen(),
            Screen::PdfViewer => self.render_pdf_screen(),
            Screen::Debug => self.render_debug_screen(),
        }
    }
    
    pub fn render_with_file_picker(&mut self, file_picker: &mut IntegratedFilePicker) -> Result<()> {
        match self.current_screen {
            Screen::Demo => self.render_demo_screen(),
            Screen::FilePicker => self.render_integrated_file_picker_screen(file_picker),
            Screen::PdfViewer => self.render_pdf_screen(),
            Screen::Debug => self.render_debug_screen(),
        }
    }
    
    fn render_demo_screen(&mut self) -> Result<()> {
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
    
    fn render_pdf_screen(&mut self) -> Result<()> {
        eprintln!("[DEBUG] render_pdf_screen called");
        // Chonker7-style split view: PDF image on left, text extraction on right
        let (width, height) = terminal::size()?;
        let split_x = width / 2;
        eprintln!("[DEBUG] Terminal size: {}x{}, split at {}", width, height, split_x);
        
        execute!(
            stdout(),
            Clear(ClearType::All),
            MoveTo(0, 0),
            Hide
        )?;
        
        // Draw a border around the PDF panel
        let (tl, tr, bl, br, h_line, v_line, _, _) = self.config.get_border_chars();
        execute!(stdout(), SetForegroundColor(self.config.get_highlight_color()))?;
        
        // Top border
        execute!(stdout(), MoveTo(0, 0), Print(tl))?;
        for i in 1..split_x - 1 {
            execute!(stdout(), MoveTo(i, 0), Print(h_line))?;
        }
        execute!(stdout(), MoveTo(split_x - 1, 0), Print(tr))?;
        
        // Side borders
        for i in 1..height - 2 {
            execute!(stdout(), MoveTo(0, i), Print(v_line))?;
            execute!(stdout(), MoveTo(split_x - 1, i), Print(v_line))?;
        }
        
        // Bottom border
        execute!(stdout(), MoveTo(0, height - 2), Print(bl))?;
        for i in 1..split_x - 1 {
            execute!(stdout(), MoveTo(i, height - 2), Print(h_line))?;
        }
        execute!(stdout(), MoveTo(split_x - 1, height - 2), Print(br))?;
        
        // Draw title
        let title = format!(" PDF Page {}/{} ", self.current_page, self.total_pages);
        execute!(
            stdout(),
            MoveTo(2, 0),
            SetForegroundColor(self.config.get_highlight_color()),
            Print(&title),
            SetForegroundColor(self.config.get_text_color())
        )?;
        
        // Use our new render_pdf_content method for Kitty protocol
        // Render in the left panel area (inside the border)
        eprintln!("[DEBUG] About to render PDF content at (1,1) size {}x{}", split_x - 2, height - 3);
        self.render_pdf_content(1, 1, split_x - 2, height - 3)?;
        eprintln!("[DEBUG] PDF content rendered");
        
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
    
    fn render_debug_screen(&mut self) -> Result<()> {
        let (width, height) = terminal::size()?;
        
        // Clear screen
        execute!(
            stdout(),
            Clear(ClearType::All),
            MoveTo(0, 0)
        )?;
        
        // Draw header
        execute!(
            stdout(),
            MoveTo(0, 0),
            SetForegroundColor(Color::Cyan),
            Print(format!("╔{}╗", "═".repeat((width - 2) as usize))),
            MoveTo(0, 1),
            Print("║"),
            MoveTo(2, 1),
            SetForegroundColor(Color::Yellow),
            Print("DEBUG OUTPUT"),
            SetForegroundColor(Color::Cyan),
            MoveTo(width - 1, 1),
            Print("║"),
            MoveTo(0, 2),
            Print(format!("╠{}╣", "═".repeat((width - 2) as usize))),
            ResetColor
        )?;
        
        // Calculate content area
        let content_start_y = 3;
        let content_height = height.saturating_sub(5); // Leave room for header and status
        
        // Display debug messages
        let visible_messages = self.debug_messages
            .iter()
            .skip(self.debug_scroll_offset)
            .take(content_height as usize);
        
        for (i, message) in visible_messages.enumerate() {
            let y_pos = content_start_y + i as u16;
            
            // Truncate message to fit screen width
            let max_width = (width - 4) as usize;
            let display_msg = if message.len() > max_width {
                format!("{}...", &message.chars().take(max_width - 3).collect::<String>())
            } else {
                message.clone()
            };
            
            // Get appropriate color for this message
            let msg_color = self.get_message_color(&message);
            
            execute!(
                stdout(),
                MoveTo(0, y_pos),
                SetForegroundColor(Color::Cyan),
                Print("║ "),
                SetForegroundColor(msg_color),
                Print(format!("{:<width$}", display_msg, width = max_width)),
                SetForegroundColor(Color::Cyan),
                MoveTo(width - 1, y_pos),
                Print("║"),
                ResetColor
            )?;
        }
        
        // Fill empty lines
        for i in self.debug_messages.len()..content_height as usize {
            let y_pos = content_start_y + i as u16;
            execute!(
                stdout(),
                MoveTo(0, y_pos),
                SetForegroundColor(Color::Cyan),
                Print("║"),
                MoveTo(width - 1, y_pos),
                Print("║"),
                ResetColor
            )?;
        }
        
        // Draw bottom border
        execute!(
            stdout(),
            MoveTo(0, height - 2),
            SetForegroundColor(Color::Cyan),
            Print(format!("╚{}╝", "═".repeat((width - 2) as usize))),
            ResetColor
        )?;
        
        // Status bar
        let status_text = format!(
            " Msgs: {} | {}-{} | ↑↓/Mouse: Scroll | PgUp/Dn | Home/End | Tab | Esc ",
            self.debug_messages.len(),
            self.debug_scroll_offset + 1,
            (self.debug_scroll_offset + content_height as usize).min(self.debug_messages.len())
        );
        
        execute!(
            stdout(),
            MoveTo(0, height - 1),
            SetAttribute(Attribute::Reverse),
            Print(format!("{:<width$}", status_text, width = width as usize)),
            SetAttribute(Attribute::NoReverse)
        )?;
        
        stdout().flush()?;
        Ok(())
    }
    
    fn render_split_mode(&mut self, width: u16, height: u16) -> Result<()> {
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
    
    fn render_full_mode(&mut self, width: u16, height: u16) -> Result<()> {
        // Full screen PDF view
        self.render_pdf_panel(0, 0, width, height - 2)?;
        Ok(())
    }
    
    fn render_minimal_mode(&self, width: u16, height: u16) -> Result<()> {
        // Just text, no borders
        self.render_text_content(0, 0, width, height - 1)?;
        Ok(())
    }
    
    fn render_pdf_panel(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
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
    
    fn render_pdf_content(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        // Force enable Kitty protocol and display image directly
        if let Some(ref image) = self.current_pdf_image {
            eprintln!("[DEBUG] Displaying PDF image at ({}, {}) with size {}x{}", x, y, width, height);
            
            // Force enable Kitty protocol
            self.kitty.force_enable();
            
            // Clear any previous image
            if let Some(image_id) = self.current_image_id {
                let _ = self.kitty.clear_image(image_id);
            }
            
            // Calculate display dimensions with better scaling
            let (img_width, img_height) = (image.width(), image.height());
            eprintln!("[DEBUG] PDF image size: {}x{}", img_width, img_height);
            
            // Better scaling calculation for beautiful display
            // Use more accurate cell dimensions (depends on terminal font)
            let cell_width = 9;  // Typical terminal font width
            let cell_height = 18; // Typical terminal font height
            
            // Calculate available space in pixels with padding
            let padding = 2; // cells of padding
            let available_width_px = ((width - padding * 2) as u32) * cell_width;
            let available_height_px = ((height - padding) as u32) * cell_height;
            
            // Calculate scale to fit with aspect ratio preservation
            let scale_x = available_width_px as f32 / img_width as f32;
            let scale_y = available_height_px as f32 / img_height as f32;
            let scale = scale_x.min(scale_y).min(1.0); // Don't upscale beyond original
            
            let display_width = (img_width as f32 * scale) as u32;
            let display_height = (img_height as f32 * scale) as u32;
            
            eprintln!("[DEBUG] Display size: {}x{} (scale: {:.2})", display_width, display_height, scale);
            
            // Center the image in the panel
            let x_offset = padding + ((width - padding * 2) - (display_width / cell_width) as u16) / 2;
            let y_offset = 1 + ((height - padding) - (display_height / cell_height) as u16) / 2;
            
            // Display the image using Kitty protocol
            match self.kitty.display_image(
                image,
                (x + x_offset) as u32,
                (y + y_offset) as u32,
                Some(display_width),
                Some(display_height),
            ) {
                Ok(image_id) => {
                    self.current_image_id = Some(image_id);
                    eprintln!("[DEBUG] Successfully displayed image with ID: {}", image_id);
                }
                Err(e) => {
                    eprintln!("[DEBUG] Failed to display image via Kitty: {}", e);
                    // Fall back to text rendering
                    self.render_pdf_content_fallback(x, y, width, height)?;
                }
            }
        } else {
            // No image available, use text rendering
            self.render_pdf_content_fallback(x, y, width, height)?;
        }
        
        Ok(())
    }
    
    fn render_pdf_content_fallback(&self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
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
        match self.current_screen {
            Screen::Debug => {
                if self.debug_scroll_offset > 0 {
                    self.debug_scroll_offset -= 1;
                }
            }
            _ => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
        }
    }
    
    pub fn scroll_down(&mut self) {
        match self.current_screen {
            Screen::Debug => {
                if self.debug_scroll_offset < self.debug_messages.len().saturating_sub(10) {
                    self.debug_scroll_offset += 1;
                }
            }
            _ => {
                if self.scroll_offset < self.pdf_content.len().saturating_sub(10) {
                    self.scroll_offset += 1;
                }
            }
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
        let next_screen = self.available_screens[next_index].clone();
        self.set_screen(next_screen);
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
        let prev_screen = self.available_screens[prev_index].clone();
        self.set_screen(prev_screen);
    }
    
    pub fn get_current_screen(&self) -> &Screen {
        &self.current_screen
    }
    
    pub fn current_screen(&self) -> &Screen {
        &self.current_screen
    }
    
    pub fn set_screen(&mut self, screen: Screen) {
        // If switching to debug screen and messages haven't been loaded yet, load them
        if screen == Screen::Debug && !self.debug_messages_loaded {
            self.load_debug_log();
            self.debug_messages_loaded = true;
        }
        self.current_screen = screen;
    }
    
    // Debug screen scrolling methods
    pub fn scroll_debug_up(&mut self) {
        if self.debug_scroll_offset > 0 {
            self.debug_scroll_offset -= 1;
        }
    }
    
    pub fn scroll_debug_down(&mut self) {
        let max_offset = self.get_debug_max_scroll_offset();
        if self.debug_scroll_offset < max_offset {
            self.debug_scroll_offset += 1;
        }
    }
    
    pub fn scroll_debug_page_up(&mut self) {
        self.debug_scroll_offset = self.debug_scroll_offset.saturating_sub(10);
    }
    
    pub fn scroll_debug_page_down(&mut self) {
        let max_offset = self.get_debug_max_scroll_offset();
        self.debug_scroll_offset = (self.debug_scroll_offset + 10).min(max_offset);
    }
    
    pub fn scroll_debug_to_top(&mut self) {
        self.debug_scroll_offset = 0;
    }
    
    pub fn scroll_debug_to_bottom(&mut self) {
        self.debug_scroll_offset = self.get_debug_max_scroll_offset();
    }
    
    fn get_debug_max_scroll_offset(&self) -> usize {
        // Calculate the visible height for debug content
        // Terminal height minus header (3 lines) and status bar (2 lines) = content height
        let terminal_height = crossterm::terminal::size().unwrap_or((80, 24)).1 as usize;
        let content_height = terminal_height.saturating_sub(5);
        
        // Maximum scroll offset is total messages minus what fits on screen
        // If all messages fit on screen, max offset is 0 (no scrolling needed)
        if self.debug_messages.len() <= content_height {
            0
        } else {
            self.debug_messages.len() - content_height
        }
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
            Screen::Debug => "Debug",
        }
    }
    
    pub fn load_pdf(&mut self, pdf_path: PathBuf) -> Result<()> {
        use crate::pdf_extraction::{DocumentAnalyzer};
        
        // Clear debug messages for new PDF load
        self.debug_messages.clear();
        self.debug_scroll_offset = 0;
        
        let msg = format!("UIRenderer::load_pdf called with: {:?}", pdf_path);
        self.add_debug_message(msg.clone());
        eprintln!("[DEBUG] {}", msg);
        
        // Load PDF page count - chonker7 style with fresh instance
        self.add_debug_message("Getting page count...".to_string());
        eprintln!("[DEBUG] Getting page count...");
        self.total_pages = content_extractor::get_page_count(&pdf_path)?;
        self.current_page = 1;
        let msg = format!("Page count: {}", self.total_pages);
        self.add_debug_message(msg.clone());
        eprintln!("[DEBUG] {}", msg);
        
        // Render first page image with high resolution for beautiful display
        self.add_debug_message("Rendering PDF page...".to_string());
        eprintln!("[DEBUG] Rendering PDF page...");
        let image = pdf_renderer::render_pdf_page(&pdf_path, 0, 2400, 3200)?;  // Higher res for crisp display
        self.add_debug_message("PDF page rendered".to_string());
        eprintln!("[DEBUG] PDF page rendered");
        
        // Use intelligent document-agnostic extraction
        self.add_debug_message("Creating analyzer...".to_string());
        eprintln!("[DEBUG] Creating analyzer...");
        let analyzer = DocumentAnalyzer::new()?;
        self.add_debug_message("Analyzing page...".to_string());
        eprintln!("[DEBUG] Analyzing page...");
        let fingerprint = analyzer.analyze_page(&pdf_path, 0)?;
        let msg = format!("Analysis complete: text={:.1}%, image={:.1}%, has_tables={}, text_quality={:.2}", 
            fingerprint.text_coverage * 100.0, 
            fingerprint.image_coverage * 100.0,
            fingerprint.has_tables,
            fingerprint.text_quality);
        self.add_debug_message(msg.clone());
        eprintln!("[DEBUG] {}", msg);
        
        // Use the intelligent ExtractionRouter to extract text
        self.add_debug_message("Starting intelligent text extraction...".to_string());
        eprintln!("[DEBUG] Starting intelligent text extraction...");
        let extraction_result = crate::pdf_extraction::ExtractionRouter::extract_with_fallback_sync(&pdf_path, 0, &fingerprint)?;
        let msg = format!("Extraction complete using method: {:?}, quality: {:.2}", 
            extraction_result.method, extraction_result.quality_score);
        self.add_debug_message(msg.clone());
        eprintln!("[DEBUG] {}", msg);
        
        // Store metadata
        self.extraction_method = Some(format!("{:?}", extraction_result.method));
        self.extraction_quality = Some(extraction_result.quality_score);
        self.extraction_timestamp = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        
        // Create metadata header with better formatting
        let filename = pdf_path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .chars()
            .take(60)
            .collect::<String>();
        
        let metadata_header = format!(
            "╔════════════════════════════════════════════════════════════════════════════════╗\n\
             ║ PDF EXTRACTION METADATA                                                        ║\n\
             ╠════════════════════════════════════════════════════════════════════════════════╣\n\
             ║ File: {:<73}║\n\
             ║ Page: {}/{:<70}║\n\
             ║ Method: {:<72}║\n\
             ║ Quality Score: {:.1}%{:<64}║\n\
             ║ Text Coverage: {:.1}%  |  Image Coverage: {:.1}%  |  Has Tables: {:<20}║\n\
             ║ Extracted: {:<68}║\n\
             ╚════════════════════════════════════════════════════════════════════════════════╝\n\n",
            filename,
            self.current_page,
            self.total_pages,
            format!("{:?}", extraction_result.method),
            extraction_result.quality_score * 100.0,
            "",
            fingerprint.text_coverage * 100.0,
            fingerprint.image_coverage * 100.0,
            if fingerprint.has_tables { "Yes" } else { "No" },
            self.extraction_timestamp.as_ref().unwrap()
        );
        
        // Combine metadata with extracted text
        let text_with_metadata = format!("{}{}", metadata_header, extraction_result.text);
        
        // Convert extracted text to grid format for display
        let text_matrix = self.text_to_matrix(&text_with_metadata, 200, 100);
        
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