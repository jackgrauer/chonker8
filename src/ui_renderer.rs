// Dynamic UI renderer that reads from hot-reloadable config
use crate::ui_config::UIConfig;
use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Attribute, Attributes, Color, Print, ResetColor, SetAttributes, SetBackgroundColor, SetForegroundColor},
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
    image_sent: bool,
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
        
        // FORCE ENABLE KITTY FOR TESTING
        kitty.force_enable();
        eprintln!("[KITTY] *** FORCE-ENABLED KITTY PROTOCOL FOR TESTING ***");
        
        // Kitty is MANDATORY for this viewer
        if kitty.is_supported() {
            eprintln!("[DEBUG] ✅ Kitty graphics protocol ACTIVE!");
        } else {
            eprintln!("[WARNING] ⚠️  Kitty not detected - PDF images require Kitty terminal!");
            eprintln!("[WARNING] Run with: kitty ./target/release/chonker8-hot [pdf]");
        }
        
        Self {
            config,
            pdf_content: vec![vec![' '; 80]; 24], // Default empty content
            current_page: 1,
            total_pages: 1,
            scroll_offset: 0,
            cursor_x: 0,
            cursor_y: 0,
            current_screen: Screen::FilePicker,
            available_screens: vec![Screen::FilePicker, Screen::PdfViewer, Screen::Debug],
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
            image_sent: false,
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
        eprintln!("[DEBUG] render() called, current_screen: {:?}", self.current_screen);
        eprintln!("[DEBUG] Has PDF image: {}", self.current_pdf_image.is_some());
        eprintln!("[DEBUG] PDF path: {:?}", self.current_pdf_path);
        let result = match self.current_screen {
            Screen::FilePicker => self.render_file_picker_screen(),
            Screen::PdfViewer => {
                eprintln!("[DEBUG] Calling render_pdf_screen()");
                self.render_pdf_screen()
            },
            Screen::Debug => self.render_debug_screen(),
        };
        eprintln!("[DEBUG] render() complete, result: {:?}", result.is_ok());
        result
    }
    
    pub fn render_with_file_picker(&mut self, file_picker: &mut IntegratedFilePicker) -> Result<()> {
        match self.current_screen {
            Screen::FilePicker => self.render_integrated_file_picker_screen(file_picker),
            Screen::PdfViewer => self.render_pdf_screen(),
            Screen::Debug => self.render_debug_screen(),
        }
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
        
        // Draw a clear vertical split line first
        execute!(stdout(), SetForegroundColor(Color::Cyan))?;
        for y in 0..height - 1 {
            execute!(stdout(), MoveTo(split_x, y), Print("│"))?;
        }
        
        // Draw panel headers
        execute!(
            stdout(),
            MoveTo(2, 0),
            SetForegroundColor(Color::Yellow),
            SetAttributes(Attributes::from(Attribute::Bold)),
            Print("◀ PDF RENDER (lopdf→vello→kitty) ▶"),
            SetAttributes(Attributes::from(Attribute::Reset))
        )?;
        
        execute!(
            stdout(),
            MoveTo(split_x + 2, 0),
            SetForegroundColor(Color::Green),
            SetAttributes(Attributes::from(Attribute::Bold)),
            Print("◀ TEXT EXTRACTION (pdftotext) ▶"),
            SetAttributes(Attributes::from(Attribute::Reset))
        )?;
        
        // Left Panel - PDF Render
        eprintln!("[DEBUG] Rendering left panel (PDF)");
        execute!(stdout(), SetForegroundColor(Color::White))?;
        
        // Show PDF status
        let pdf_status = format!(" Page {}/{} ", self.current_page, self.total_pages);
        execute!(
            stdout(),
            MoveTo(2, 1),
            SetForegroundColor(Color::DarkYellow),
            Print(&pdf_status),
            SetForegroundColor(Color::White)
        )?;
        
        // Render PDF content or image
        if self.current_pdf_image.is_some() {
            eprintln!("[DEBUG] Have PDF image, attempting Kitty display");
            let (img_w, img_h) = self.current_pdf_image.as_ref().map(|i| (i.width(), i.height())).unwrap();
            eprintln!("[DEBUG] Image dimensions: {}x{}", img_w, img_h);
            eprintln!("[DEBUG] Kitty supported: {}", self.kitty.is_supported());
            eprintln!("[DEBUG] Display area: x=2, y=2, width={}, height={}", split_x - 4, height - 4);
            
            // Try to display the PDF image in the left panel
            self.render_pdf_content(2, 2, split_x - 4, height - 4)?;
            
            // Also show some debug info on screen
            execute!(
                stdout(),
                MoveTo(2, height - 3),
                SetForegroundColor(Color::DarkGrey),
                Print(format!("PDF: {}x{}", 
                    self.current_pdf_image.as_ref().map(|i| i.width()).unwrap_or(0),
                    self.current_pdf_image.as_ref().map(|i| i.height()).unwrap_or(0)
                )),
                SetForegroundColor(Color::White)
            )?;
        } else {
            eprintln!("[DEBUG] No PDF image loaded!");
            execute!(
                stdout(),
                MoveTo(2, 5),
                SetForegroundColor(Color::Red),
                Print("[ERROR: No PDF image loaded]")
            )?;
        }
        eprintln!("[DEBUG] Left panel rendered");
        
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
            SetAttributes(Attributes::from(Attribute::Reverse)),
            Print(format!("{:<width$}", status_text, width = width as usize)),
            SetAttributes(Attributes::from(Attribute::Reset))
        )?;
        
        stdout().flush()?;
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
        
        // Draw title with rendering method indicator and scroll info
        let title = format!(" 📄 PDF Page {}/{} [lopdf-vello-kitty] Scroll: {} ", 
                          self.current_page, self.total_pages, self.scroll_offset);
        execute!(
            stdout(),
            MoveTo(x + 2, y),
            SetBackgroundColor(Color::Rgb { r: 30, g: 30, b: 40 }),
            SetForegroundColor(Color::Rgb { r: 100, g: 200, b: 255 }),
            Print(&title),
            ResetColor
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
        
        // Draw title with extraction method indicator
        let title = " 📝 Extracted Text [pdftotext] ";
        execute!(
            stdout(),
            MoveTo(x + 2, y),
            SetBackgroundColor(Color::Rgb { r: 30, g: 30, b: 40 }),
            SetForegroundColor(Color::Rgb { r: 255, g: 200, b: 100 }),
            Print(title),
            ResetColor
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
    
    
    fn render_pdf_content(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        // ALWAYS use Kitty protocol - NO FALLBACK
        if let Some(ref image) = self.current_pdf_image {
            // Only send the image ONCE to avoid slowdown
            if self.image_sent {
                return Ok(()); // Image already sent, skip
            }
            
            eprintln!("[DEBUG] Sending Kitty image (first time only)");
            self.image_sent = true;
            
            // Move image further down into the visible area
            // Start at y+10 to ensure it's well within the panel
            let image_y = y + 10;
            let image_x = x + 2;
            
            // Move cursor to position
            execute!(
                stdout(),
                MoveTo(image_x, image_y)
            )?;
            
            // Use inline Kitty implementation with correct protocol
            struct KittyImage;
            impl KittyImage {
                fn send_image_positioned(image: &DynamicImage, width: u32, height: u32, x: u16, y: u16) -> Result<()> {
                    // Convert to PNG
                    let mut png_data = Vec::new();
                    image.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)?;
                    
                    // Base64 encode
                    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
                    let encoded = BASE64.encode(&png_data);
                    
                    // First, clear the terminal properly
                    std::io::stdout().flush()?;
                    
                    // Move cursor to position using crossterm, not raw escape codes
                    execute!(stdout(), MoveTo(x, y))?;
                    
                    // Clear any existing images - use raw bytes to ensure proper escape
                    use std::io::Write;
                    let clear_cmd = b"\x1b_Ga=d\x1b\\";
                    std::io::stdout().write_all(clear_cmd)?;
                    std::io::stdout().flush()?;
                    
                    // Kitty protocol requires chunking for large images
                    // Maximum chunk size is 4096 bytes
                    const CHUNK_SIZE: usize = 4096;
                    let encoded_bytes = encoded.as_bytes();
                    
                    if encoded_bytes.len() <= CHUNK_SIZE {
                        // Small image, send in one go
                        let mut cmd = Vec::new();
                        cmd.extend_from_slice(b"\x1b_Ga=T,f=100,s=");
                        cmd.extend_from_slice(width.to_string().as_bytes());
                        cmd.extend_from_slice(b",v=");
                        cmd.extend_from_slice(height.to_string().as_bytes());
                        cmd.extend_from_slice(b";");
                        cmd.extend_from_slice(encoded_bytes);
                        cmd.extend_from_slice(b"\x1b\\");
                        
                        std::io::stdout().write_all(&cmd)?;
                    } else {
                        // Large image, send in chunks
                        let chunks: Vec<&[u8]> = encoded_bytes.chunks(CHUNK_SIZE).collect();
                        
                        for (i, chunk) in chunks.iter().enumerate() {
                            let mut cmd = Vec::new();
                            cmd.extend_from_slice(b"\x1b_G");
                            
                            if i == 0 {
                                // First chunk has the full header
                                cmd.extend_from_slice(b"a=T,f=100,s=");
                                cmd.extend_from_slice(width.to_string().as_bytes());
                                cmd.extend_from_slice(b",v=");
                                cmd.extend_from_slice(height.to_string().as_bytes());
                                cmd.extend_from_slice(b",m=1;");
                            } else if i == chunks.len() - 1 {
                                // Last chunk
                                cmd.extend_from_slice(b"m=0;");
                            } else {
                                // Middle chunks
                                cmd.extend_from_slice(b"m=1;");
                            }
                            
                            cmd.extend_from_slice(chunk);
                            cmd.extend_from_slice(b"\x1b\\");
                            
                            std::io::stdout().write_all(&cmd)?;
                            std::io::stdout().flush()?;
                        }
                        
                        eprintln!("[KITTY] Sent large image in {} chunks", chunks.len());
                    }
                    
                    std::io::stdout().flush()?;
                    
                    eprintln!("[KITTY] Sent image {}x{} at ({},{}), {} bytes encoded", 
                             width, height, x, y, encoded.len());
                    
                    Ok(())
                }
            }
            
            // Scale the image for display - smaller to reduce lag
            let scale = 0.1; // 10% of original size to reduce data transfer
            let display_width = (image.width() as f32 * scale) as u32;
            let display_height = (image.height() as f32 * scale) as u32;
            
            eprintln!("[DEBUG] Original: {}x{}, Display: {}x{}, Position: ({}, {})", 
                     image.width(), image.height(), display_width, display_height, image_x, image_y);
            
            // Send image at fixed position within panel
            match KittyImage::send_image_positioned(image, display_width, display_height, image_x, image_y) {
                Ok(_) => {
                    eprintln!("[DEBUG] ✅ KITTY IMAGE SENT SUCCESSFULLY!");
                }
                Err(e) => {
                    eprintln!("[ERROR] KITTY FAILED: {}", e);
                    eprintln!("[ERROR] This viewer REQUIRES Kitty-compatible terminal!");
                    
                    // Show error message on screen
                    execute!(
                        stdout(),
                        MoveTo(x + 2, y + height/2),
                        SetForegroundColor(Color::Red),
                        Print("⚠️  KITTY ERROR ⚠️"),
                        MoveTo(x + 2, y + height/2 + 2),
                        Print(&format!("Error: {}", e)),
                        SetForegroundColor(Color::White)
                    )?;
                }
            }
        } else {
            // No image - but this shouldn't happen
            eprintln!("[ERROR] No PDF image loaded!");
            execute!(
                stdout(),
                MoveTo(x + 2, y + height/2),
                SetForegroundColor(Color::Red),
                Print("⚠️  NO PDF IMAGE ⚠️"),
                SetForegroundColor(Color::White)
            )?;
        }
        
        Ok(())
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
            crossterm::style::SetAttributes(crossterm::style::Attributes::from(crossterm::style::Attribute::Reverse)),
            Print(" ".repeat(width as usize)),
            crossterm::style::SetAttributes(crossterm::style::Attributes::from(crossterm::style::Attribute::Reset))
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
                // Larger scroll steps for PDF image viewing
                if self.scroll_offset > 0 {
                    self.scroll_offset = self.scroll_offset.saturating_sub(5);
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
                // Larger scroll steps for PDF image viewing (up to 100 to see off-screen images)
                if self.scroll_offset < 100 {
                    self.scroll_offset = (self.scroll_offset + 5).min(100);
                }
            }
        }
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
            Screen::FilePicker => "File Picker", 
            Screen::PdfViewer => "PDF Viewer",
            Screen::Debug => "Debug",
        }
    }
    
    pub fn load_pdf(&mut self, pdf_path: PathBuf) -> Result<()> {
        use crate::pdf_extraction::{DocumentAnalyzer, PageFingerprint};
        
        // Clear debug messages for new PDF load
        self.debug_messages.clear();
        self.debug_scroll_offset = 0;
        self.image_sent = false; // Reset flag for new PDF
        
        let msg = format!("A-B Comparison: Loading PDF {:?}", pdf_path);
        eprintln!("[INFO] Left pane: lopdf-vello-kitty rendering");
        eprintln!("[INFO] Right pane: pdftotext extraction");
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
        self.add_debug_message("Rendering PDF with lopdf-vello-kitty...".to_string());
        eprintln!("[DEBUG] Rendering PDF with Vello GPU acceleration...");
        let mut image = pdf_renderer::render_pdf_page(&pdf_path, 0, 2400, 3200)?;  // Higher res for crisp display
        
        // Apply dark mode filter for better visibility
        image = self.apply_dark_mode_filter(image);
        self.add_debug_message("PDF page rendered".to_string());
        eprintln!("[DEBUG] PDF page rendered");
        
        // Use intelligent document-agnostic extraction - with fallback
        self.add_debug_message("Creating analyzer...".to_string());
        eprintln!("[DEBUG] Creating analyzer...");
        
        let fingerprint = match DocumentAnalyzer::new() {
            Ok(analyzer) => {
                self.add_debug_message("Analyzing page...".to_string());
                eprintln!("[DEBUG] Analyzing page...");
                match analyzer.analyze_page(&pdf_path, 0) {
                    Ok(fp) => {
                        let msg = format!("Analysis complete: text={:.1}%, image={:.1}%, has_tables={}, text_quality={:.2}", 
                            fp.text_coverage * 100.0, 
                            fp.image_coverage * 100.0,
                            fp.has_tables,
                            fp.text_quality);
                        self.add_debug_message(msg.clone());
                        eprintln!("[DEBUG] {}", msg);
                        fp
                    }
                    Err(e) => {
                        eprintln!("[WARNING] Analysis failed: {}, using defaults", e);
                        PageFingerprint::new()
                    }
                }
            }
            Err(e) => {
                eprintln!("[WARNING] Analyzer creation failed: {}, using defaults", e);
                PageFingerprint::new()
            }
        };
        
        // Extract text using pdftotext for the right panel
        self.add_debug_message("Extracting text with pdftotext...".to_string());
        eprintln!("[DEBUG] Running pdftotext with layout preservation...");
        
        let extraction_result = match std::process::Command::new("pdftotext")
            .args(&[
                "-layout",  // Preserve layout
                "-nopgbrk", // No page breaks
                "-f", "1",  // First page
                "-l", "1",  // Last page
                pdf_path.to_str().unwrap(),
                "-"  // Output to stdout
            ])
            .output() {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                eprintln!("[DEBUG] pdftotext extracted {} characters", text.len());
                crate::pdf_extraction::ExtractionResult {
                    text,
                    quality_score: 0.8,
                    method: crate::pdf_extraction::ExtractionMethod::PdfToText,
                    extraction_time_ms: 0,
                }
            }
            _ => {
                eprintln!("[WARNING] pdftotext failed, using fallback");
                crate::pdf_extraction::ExtractionResult {
                    text: "Text extraction failed - pdftotext not available".to_string(),
                    quality_score: 0.0,
                    method: crate::pdf_extraction::ExtractionMethod::PdfToText,
                    extraction_time_ms: 0,
                }
            }
        };
        
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
    
    /// Apply dark mode filter to PDF image for better visibility in terminal
    fn apply_dark_mode_filter(&self, image: DynamicImage) -> DynamicImage {
        use image::{ImageBuffer, Rgba};
        
        let rgba_image = image.to_rgba8();
        let (width, height) = rgba_image.dimensions();
        let mut buffer = ImageBuffer::new(width, height);
        
        for y in 0..height {
            for x in 0..width {
                let pixel = rgba_image.get_pixel(x, y);
                let Rgba([r, g, b, a]) = *pixel;
            
            // Calculate luminance
            let lum = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as u8;
            
            // Smart inversion for dark mode
            let (new_r, new_g, new_b) = if lum > 240 {
                // White background -> dark background
                (25, 25, 35)
            } else if lum > 200 {
                // Light gray -> darker
                (45, 45, 55)
            } else if lum < 40 {
                // Black text -> bright text
                (230, 230, 240)
            } else {
                // Enhance contrast for mid-tones
                let factor = if lum < 128 { 1.6 } else { 0.6 };
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