// Dynamic UI renderer that reads from hot-reloadable config
use crate::core::config::UIConfig;
use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{stdout, Write};
use std::path::PathBuf;
use image::DynamicImage;
use crate::display::file_browser::IntegratedFilePicker;
use crate::pdf::page_renderer as pdf_renderer;
use crate::display::kitty_graphics::KittyProtocol;
use crate::display::mouse_zones::{MouseHandler, MouseZone, PanelFocus, ScrollAction, ZoomAction};
use crate::display::viewport::ViewportManager;
// Use the Grid module from the same module
use super::grid::Grid;


#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    FilePicker,
    PdfViewer,
}

pub struct UIRenderer {
    config: UIConfig,
    pdf_content: Vec<Vec<char>>,
    grid: Grid,  // Excel-style editable grid
    current_page: usize,
    total_pages: usize,
    scroll_offset: usize,
    cursor_x: usize,
    cursor_y: usize,
    current_screen: Screen,
    available_screens: Vec<Screen>,
    file_picker: Option<IntegratedFilePicker>,
    pub current_pdf_path: Option<PathBuf>,
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
    first_render: bool,
    right_panel_dirty: bool,  // Track when right panel needs full redraw
    last_split_x: u16,  // Track if window was resized
    mouse_handler: MouseHandler,  // Mouse zone detection and handling
    pdf_zoom: f32,  // PDF panel zoom level
    text_zoom: f32,  // Text editor zoom level
    viewports: ViewportManager,  // Manage separate render regions
    last_pdf_title: Option<String>,  // Cache the PDF title to avoid re-rendering
}

impl UIRenderer {
    // Terminal drawing helper methods
    fn draw_header(&self, x: u16, y: u16, text: &str) -> Result<()> {
        execute!(
            stdout(),
            MoveTo(x, y),
            SetBackgroundColor(Color::DarkBlue),
            SetForegroundColor(Color::White),
            Print(format!(" {} ", text)),
            ResetColor
        )?;
        Ok(())
    }
    
    fn clear_region(&self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        execute!(stdout(), SetBackgroundColor(Color::Black))?;
        let blank_line = " ".repeat(width as usize);
        for row in 0..height {
            execute!(
                stdout(),
                MoveTo(x, y + row),
                Print(&blank_line)
            )?;
        }
        execute!(stdout(), ResetColor)?;
        Ok(())
    }
    
    pub fn new(config: UIConfig) -> Self {
        // Initialize the file picker
        let file_picker = match IntegratedFilePicker::new() {
            Ok(picker) => Some(picker),
            Err(e) => {
                // Silenced: eprintln!("Warning: Failed to initialize file picker: {}", e);
                None
            }
        };
        
        let mut kitty = KittyProtocol::new();
        
        // FORCE ENABLE KITTY FOR TESTING
        kitty.force_enable();
        // eprintln!("[KITTY] *** FORCE-ENABLED KITTY PROTOCOL FOR TESTING ***");
        
        // Kitty is MANDATORY for this viewer
        if kitty.is_supported() {
            // eprintln!("[DEBUG] Kitty graphics protocol ACTIVE");
        } else {
            // Silenced: eprintln!("[WARNING] Kitty not detected - PDF images require Kitty terminal");
            // Silenced: eprintln!("[WARNING] Run with: kitty ./target/release/chonker8-hot [pdf]");
        }
        
        // Calculate actual available width for Excel grid based on terminal size
        let (term_width, term_height) = terminal::size().unwrap_or((80, 24));
        let grid_width = (term_width / 2 - 4) as usize; // Half terminal minus borders
        
        Self {
            config,
            pdf_content: vec![vec![' '; 80]; 24], // Default empty content
            grid: Grid::new(grid_width.max(40), 50),  // Initialize Excel grid with actual width
            current_page: 1,
            total_pages: 1,
            scroll_offset: 0,
            cursor_x: 0,
            cursor_y: 0,
            current_screen: Screen::FilePicker,
            available_screens: vec![Screen::FilePicker, Screen::PdfViewer],
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
            first_render: true,
            right_panel_dirty: true,
            last_split_x: 0,
            mouse_handler: MouseHandler::new(),
            pdf_zoom: 1.0,
            text_zoom: 1.0,
            viewports: ViewportManager::new(term_width, term_height),
            last_pdf_title: None,
        }
    }
    
    pub fn update_config(&mut self, config: UIConfig) {
        self.config = config;
    }
    
    pub fn set_pdf_content(&mut self, content: Vec<Vec<char>>) {
        self.pdf_content = content;
        self.viewports.text_viewport.mark_dirty();
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
    
    
    
    pub fn render(&mut self) -> Result<()> {
        match self.current_screen {
            Screen::FilePicker => self.render_file_picker_screen(),
            Screen::PdfViewer => self.render_pdf_screen(),
        }
    }
    
    pub fn render_with_file_picker(&mut self, file_picker: &mut IntegratedFilePicker) -> Result<()> {
        match self.current_screen {
            Screen::FilePicker => self.render_integrated_file_picker_screen(file_picker),
            Screen::PdfViewer => self.render_pdf_screen(),
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
                Print("[!] File picker not available - using fallback"),
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
        // Get terminal dimensions
        let (width, height) = terminal::size()?;
        let split_x = width / 2;
        
        // Update mouse handler dimensions
        self.mouse_handler.update_dimensions(width, height);
        
        // Check if window was resized
        if split_x != self.last_split_x {
            self.viewports.resize(width, height);
            self.right_panel_dirty = true;
            self.image_sent = false;  // Re-render everything on resize
            self.first_render = true;  // Treat resize as a new render to clear artifacts
            self.last_split_x = split_x;
        }
        
        // Hide cursor at the start to prevent flickering
        execute!(stdout(), crossterm::cursor::Hide)?;
        
        // Only clear screen on first render
        if self.first_render {
            execute!(
                stdout(),
                Clear(ClearType::All),
                MoveTo(0, 0)
            )?;
            self.first_render = false;
            self.viewports.pdf_viewport.mark_dirty();
            self.viewports.text_viewport.mark_dirty();
            self.viewports.status_viewport.mark_dirty();
        }
        
        // Clear dirty viewports before rendering
        self.viewports.clear_dirty()?;
        
        // Only draw separator and headers when they need updating
        if self.viewports.headers_need_redraw() || !self.image_sent {
            // Draw vertical separator
            self.viewports.draw_separator()?;
            
            // Only update the header if the PDF title changed
            let current_title = if let Some(ref path) = self.current_pdf_path {
                Some(format!("PDF: {}", path.file_name().unwrap_or_default().to_string_lossy()))
            } else {
                Some("PDF DOCUMENT".to_string())
            };
            
            // Only redraw if title changed or first render
            if self.last_pdf_title != current_title || !self.image_sent {
                // Clear top line for headers
                execute!(
                    stdout(),
                    MoveTo(0, 0),
                    Clear(ClearType::CurrentLine)
                )?;
                
                // Panel titles with active indication
                let pdf_color = self.mouse_handler.get_panel_highlight_color(PanelFocus::Pdf);
                let text_color = self.mouse_handler.get_panel_highlight_color(PanelFocus::Text);
                
                // PDF panel header
                execute!(
                    stdout(),
                    MoveTo(2, 0),
                    SetForegroundColor(pdf_color),
                    Print(if self.mouse_handler.active_panel == PanelFocus::Pdf { 
                        "● " 
                    } else { 
                        "○ " 
                    }),
                    Print(&current_title.as_ref().unwrap()),
                    ResetColor
                )?;
                
                // Text editor header (only when focus changes)
                execute!(
                    stdout(),
                    MoveTo(split_x + 2, 0),
                    SetForegroundColor(text_color),
                    Print(if self.mouse_handler.active_panel == PanelFocus::Text { 
                        "● TEXT EDITOR" 
                    } else { 
                        "○ TEXT EDITOR" 
                    }),
                    ResetColor
                )?;
                
                self.last_pdf_title = current_title;
            }
            
            self.viewports.mark_headers_clean();
        }
        
        // Render PDF content only if viewport is dirty
        if self.viewports.pdf_viewport.is_dirty() || !self.image_sent {
            if self.current_pdf_image.is_some() {
                // Use viewport dimensions for PDF
                let vp = &self.viewports.pdf_viewport;
                self.render_pdf_content(vp.x, vp.y, vp.width, vp.height)?;
            } else {
                execute!(
                    stdout(),
                    MoveTo(2, 5),
                    SetForegroundColor(Color::Red),
                    Print("ERROR: No PDF image loaded"),
                    ResetColor
                )?;
            }
            self.viewports.pdf_viewport.mark_clean();
        }
        
        // Render text content only if viewport is dirty
        if self.viewports.text_viewport.is_dirty() || self.right_panel_dirty {
            // Show extraction method in a clean way
            if let Some(method) = &self.extraction_method {
                let vp = &self.viewports.text_viewport;
                execute!(
                    stdout(),
                    MoveTo(vp.x + 1, vp.y),
                    SetForegroundColor(Color::DarkGrey),
                    Print(format!("[{}]", method.to_uppercase())),
                    ResetColor
                )?;
            }
            
            // Render text content using viewport dimensions
            let vp = &self.viewports.text_viewport;
            self.render_text_content(vp.x, vp.y + 1, vp.width, vp.height - 1)?;
            
            self.viewports.text_viewport.mark_clean();
            self.right_panel_dirty = false;
        }
        
        // Render status bar only if viewport is dirty
        if self.viewports.status_viewport.is_dirty() {
            let status_parts: Vec<String> = vec![
                // File info or default text
                self.current_pdf_path.as_ref().map(|path| 
                    format!("File: {} │ Page {}/{}", 
                        path.file_name().unwrap_or_default().to_string_lossy(),
                        self.current_page, 
                        self.total_pages)
                ).unwrap_or_else(|| "PDF - TEST Screen".to_string()),
                
                // Excel grid status or shortcuts
                self.grid.get_status_message()
                    .map(|msg| msg.to_string())
                    .unwrap_or_else(|| "F1:Help Ctrl-F:Find Ctrl-C:Copy Ctrl-X:Cut Ctrl-V:Paste".to_string()),
                
                // Navigation help
                "TAB:Switch ESC:Exit".to_string(),
            ];
            
            let status_text = status_parts.join(" │ ");
            
            let vp = &self.viewports.status_viewport;
            execute!(
                stdout(),
                MoveTo(vp.x, vp.y),
                SetBackgroundColor(Color::DarkBlue),
                SetForegroundColor(Color::White),
                Print(format!(" {:<width$} ", status_text, width = vp.width as usize - 2)),
                ResetColor
            )?;
            
            self.viewports.status_viewport.mark_clean();
        }
        
        // Position cursor in the right panel at the Excel grid cursor position
        // Account for line numbers (5 chars) if enabled
        // (Cursor position calculation removed - we use colored backgrounds instead)
        
        // Final cleanup: ALWAYS clear columns 0 and 1 to prevent any artifacts
        self.clear_region(0, 0, 2, height)?;
        
        // Keep cursor hidden - we use colored backgrounds to show cursor position
        // This prevents the double cursor visual issue
        
        stdout().flush()?;
        Ok(())
    }
    
    /*
    // Debug screen removed - not needed
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
    */
    
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
        
        // Draw title with clean DOS-style formatting
        let title = format!(" PAGE {}/{} ", self.current_page, self.total_pages);
        execute!(
            stdout(),
            MoveTo(x + 2, y),
            SetBackgroundColor(Color::DarkBlue),
            SetForegroundColor(Color::White),
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
        let default_method = "pdftotext".to_string();
        let method = self.extraction_method.as_ref().unwrap_or(&default_method);
        let title = format!(" 📝 Extracted Text [{}] ", method);
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
        
        // Don't manage cursor visibility here - it's handled globally
        // Just position the cursor where it should be for the text editor
        if self.config.panels.text.show_cursor {
            execute!(
                stdout(),
                MoveTo(content_x + self.cursor_x as u16, content_y + self.cursor_y as u16)
            )?;
        }
        
        Ok(())
    }
    
    
    fn render_pdf_content(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        // Only clear and redraw if we haven't sent the image yet or on resize
        if !self.image_sent {
            // First, clear the ENTIRE left half with black background
            let split_x = width + x;  // This should be the split position
            execute!(stdout(), SetBackgroundColor(Color::Black))?;
            for row in 0..height {
                execute!(
                    stdout(),
                    MoveTo(0, y + row),  // Always start from column 0
                    Print(" ".repeat(split_x as usize))  // Clear entire left half
                )?;
            }
            execute!(stdout(), ResetColor)?;
        }
        
        // Only attempt Kitty protocol if supported
        if let Some(ref image) = self.current_pdf_image {
            // Check if Kitty protocol is actually supported
            let kitty_supported = std::env::var("KITTY_WINDOW_ID").is_ok() || 
                                 std::env::var("TERM").unwrap_or_default().contains("kitty");
            
            if !kitty_supported {
                // Fallback: Show message instead of broken escape sequences
                if !self.image_sent {
                    self.image_sent = true;
                    execute!(
                        stdout(),
                        MoveTo(x + 5, y + height/2),
                        SetForegroundColor(Color::DarkGrey),
                        Print("[PDF rendering requires Kitty terminal]"),
                        ResetColor
                    )?;
                }
                return Ok(());
            }
            
            // Only send the image if we haven't sent it yet or if screen was cleared
            if !self.image_sent {
            
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
                    
                    // Move cursor to position using crossterm, not raw escape codes
                    execute!(stdout(), MoveTo(x, y))?;
                    
                    // Clear any existing images - use raw bytes to ensure proper escape
                    use std::io::Write;
                    let clear_cmd = b"\x1b_Ga=d\x1b\\";
                    std::io::stdout().write_all(clear_cmd)?;
                    
                    // Kitty protocol requires chunking for large images
                    // Maximum chunk size is 4096 bytes
                    const CHUNK_SIZE: usize = 4096;
                    let encoded_bytes = encoded.as_bytes();
                    
                    if encoded_bytes.len() <= CHUNK_SIZE {
                        // Small image, send in one go
                        // Use c= and r= for cell dimensions (what we want to display in)
                        let mut cmd = Vec::new();
                        cmd.extend_from_slice(b"\x1b_Ga=T,f=100,c=");
                        cmd.extend_from_slice(width.to_string().as_bytes());
                        cmd.extend_from_slice(b",r=");
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
                                // Use c= and r= for cell dimensions
                                cmd.extend_from_slice(b"a=T,f=100,c=");
                                cmd.extend_from_slice(width.to_string().as_bytes());
                                cmd.extend_from_slice(b",r=");
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
                        }
                        
                    }
                    
                    // Final flush after sending all image data
                    std::io::stdout().flush()?;
                    
                    Ok(())
                }
            }
            
            // Calculate aspect-ratio preserved dimensions with ZOOM support
            let panel_width_cells = width as u32;
            let panel_height_cells = height as u32;
            
            // Get actual image dimensions to preserve aspect ratio
            let img_width = image.width() as f32;
            let img_height = image.height() as f32;
            let aspect_ratio = img_width / img_height;
            
            // Apply zoom factor to dimensions
            let zoom_factor = self.pdf_zoom;
            
            // Calculate the best fit while preserving aspect ratio
            // Always use the full available dimension and scale the other accordingly
            let panel_aspect = panel_width_cells as f32 / panel_height_cells as f32;
            
            let (base_width, base_height) = if panel_aspect > aspect_ratio {
                // Panel is wider than image - scale to full height
                let display_height = panel_height_cells;
                let display_width = (display_height as f32 * aspect_ratio).round() as u32;
                // Make sure we don't exceed panel width
                if display_width > panel_width_cells {
                    (panel_width_cells, (panel_width_cells as f32 / aspect_ratio).round() as u32)
                } else {
                    (display_width, display_height)
                }
            } else {
                // Panel is taller than image - scale to full width
                let display_width = panel_width_cells;
                let display_height = (display_width as f32 / aspect_ratio).round() as u32;
                // Make sure we don't exceed panel height
                if display_height > panel_height_cells {
                    ((panel_height_cells as f32 * aspect_ratio).round() as u32, panel_height_cells)
                } else {
                    (display_width, display_height)
                }
            };
            
            // Apply zoom to the dimensions
            let display_width = ((base_width as f32 * zoom_factor) as u32).max(10).min(panel_width_cells * 3);
            let display_height = ((base_height as f32 * zoom_factor) as u32).max(10).min(panel_height_cells * 3);
            
            // Apply scroll offsets from mouse handler
            let scroll_x = (self.mouse_handler.pdf_scroll_x as i32).max(-(display_width as i32)).min(display_width as i32);
            let scroll_y = (self.mouse_handler.pdf_scroll_y as i32).max(-(display_height as i32)).min(display_height as i32);
            
            // Calculate position with scroll offsets
            let x_offset = (panel_width_cells.saturating_sub(display_width) / 2) as i32 - scroll_x;
            let y_offset = (panel_height_cells.saturating_sub(display_height) / 2) as i32 - scroll_y;
            
            // Position at centered location within panel (clamp to valid range)
            let image_x = (x as i32 + x_offset).max(0) as u16;
            let image_y = (y as i32 + y_offset).max(0) as u16;
            
            // Move cursor to position
            execute!(
                stdout(),
                MoveTo(image_x, image_y)
            )?;
            
                // Send image at fixed position within panel
                match KittyImage::send_image_positioned(image, display_width, display_height, image_x, image_y) {
                    Ok(_) => {
                        self.image_sent = true;  // Mark that we've sent it (for tracking purposes)
                    }
                    Err(_e) => {
                        // Silently fail - don't clutter the display
                    }
                }
            }
        } else {
            // No image - silently handle
        }
        
        Ok(())
    }
    
    
    
    fn render_text_content(&self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        // Apply scrolling offsets to the grid rendering
        let scroll_y = self.grid.scroll_y;
        let scroll_x = self.grid.scroll_x;
        
        // Apply text zoom (affects font size conceptually, but we'll use it for spacing)
        let zoom = self.text_zoom;
        let row_height = if zoom > 1.5 { 2 } else { 1 }; // Double-spaced when zoomed in
        
        // Render the Excel grid with block selection and scrolling
        let visible_rows = height / row_height;
        for display_row in 0..visible_rows.min(self.grid.cells.len().saturating_sub(scroll_y) as u16) {
            let grid_row = (display_row + scroll_y as u16) as usize;
            let screen_y = y + (display_row * row_height);
            
            // Skip if row is out of bounds
            if grid_row >= self.grid.cells.len() || screen_y >= y + height {
                break;
            }
            
            // Build entire row first, then print it all at once
            let mut row_output = String::new();
            execute!(stdout(), MoveTo(x, screen_y))?;
            
            // Line numbers
            if self.config.panels.text.line_numbers && width > 5 {
                execute!(
                    stdout(),
                    SetForegroundColor(Color::DarkGrey),
                    Print(format!("{:4}│", grid_row + 1)),  // Line numbers start at 1
                    ResetColor,
                )?;
                
                // Grid content - build entire row as string first
                let text_start = 5;
                let text_width = width - text_start;
                
                // Build the row string with horizontal scrolling
                for display_col in 0..text_width.min(self.grid.width as u16) {
                    let grid_col = (display_col as usize) + scroll_x;
                    
                    let ch = if grid_row < self.grid.cells.len() && grid_col < self.grid.cells[grid_row].len() {
                        self.grid.cells[grid_row][grid_col]
                    } else {
                        ' '
                    };
                    
                    row_output.push(ch);
                    
                    // Add extra spacing when zoomed in
                    if zoom > 1.5 && display_col < text_width - 1 {
                        row_output.push(' ');
                    }
                }
                
                // Pad the rest of the row with spaces to clear any old content
                while row_output.len() < text_width as usize {
                    row_output.push(' ');
                }
                
                // Print the entire row at once
                execute!(stdout(), Print(&row_output))?;
                
                // Now handle selection and cursor highlighting
                // grid_row already defined above
                
                // First, highlight any selected cells in this row
                if self.grid.selecting {
                    let (x1, y1, x2, y2) = self.grid.get_selection_bounds();
                    if grid_row >= y1 && grid_row <= y2 {
                        // This row has selected cells
                        for col in x1..=x2 {
                            if col < text_width as usize {
                                execute!(
                                    stdout(),
                                    MoveTo(x + text_start + col as u16, screen_y),
                                    SetBackgroundColor(Color::Blue),
                                    SetForegroundColor(Color::White),
                                )?;
                                
                                let ch = if grid_row < self.grid.cells.len() && col < self.grid.cells[grid_row].len() {
                                    self.grid.cells[grid_row][col]
                                } else {
                                    ' '
                                };
                                
                                execute!(stdout(), Print(ch), ResetColor)?;
                            }
                        }
                    }
                }
                
                // Highlight search results
                if self.grid.searching && !self.grid.search_query.is_empty() {
                    for display_col in 0..text_width as usize {
                        let grid_col = display_col + scroll_x;
                        if self.grid.is_search_match(grid_col, grid_row) {
                            execute!(
                                stdout(),
                                MoveTo(x + text_start + display_col as u16, screen_y),
                                SetBackgroundColor(Color::Yellow),
                                SetForegroundColor(Color::Black),
                            )?;
                            
                            let ch = if grid_row < self.grid.cells.len() && grid_col < self.grid.cells[grid_row].len() {
                                self.grid.cells[grid_row][grid_col]
                            } else {
                                ' '
                            };
                            
                            execute!(stdout(), Print(ch), ResetColor)?;
                        }
                    }
                }
                
                // Then highlight the cursor (overwrites selection if at same position)
                if grid_row == self.grid.cursor.1 {
                    let cursor_col = self.grid.cursor.0;
                    // Check if cursor is visible with horizontal scrolling
                    if cursor_col >= scroll_x && cursor_col < scroll_x + text_width as usize {
                        let display_col = cursor_col - scroll_x;
                        execute!(
                            stdout(),
                            MoveTo(x + text_start + display_col as u16, screen_y),
                            SetBackgroundColor(Color::DarkBlue),
                            SetForegroundColor(Color::White),
                        )?;
                        
                        let ch = if grid_row < self.grid.cells.len() && cursor_col < self.grid.cells[grid_row].len() {
                            self.grid.cells[grid_row][cursor_col]
                        } else {
                            ' '
                        };
                        
                        execute!(stdout(), Print(ch), ResetColor)?;
                    }
                }
            } else {
                // No line numbers - build entire row as string
                for display_col in 0..width.min(self.grid.width as u16) {
                    let grid_col = (display_col as usize) + scroll_x;
                    
                    let ch = if grid_row < self.grid.cells.len() && grid_col < self.grid.cells[grid_row].len() {
                        self.grid.cells[grid_row][grid_col]
                    } else {
                        ' '
                    };
                    
                    row_output.push(ch);
                }
                
                // Pad the rest of the row
                while row_output.len() < width as usize {
                    row_output.push(' ');
                }
                
                // Print the entire row at once
                execute!(stdout(), Print(&row_output))?;
                
                // Handle selection and cursor highlighting
                // grid_row already defined above
                
                // First, highlight any selected cells in this row
                if self.grid.selecting {
                    let (x1, y1, x2, y2) = self.grid.get_selection_bounds();
                    if grid_row >= y1 && grid_row <= y2 {
                        // This row has selected cells
                        for col in x1..=x2 {
                            if col < width as usize {
                                execute!(
                                    stdout(),
                                    MoveTo(x + col as u16, screen_y),
                                    SetBackgroundColor(Color::Blue),
                                    SetForegroundColor(Color::White),
                                )?;
                                
                                let ch = if grid_row < self.grid.cells.len() && col < self.grid.cells[grid_row].len() {
                                    self.grid.cells[grid_row][col]
                                } else {
                                    ' '
                                };
                                
                                execute!(stdout(), Print(ch), ResetColor)?;
                            }
                        }
                    }
                }
                
                // Then highlight the cursor
                if grid_row == self.grid.cursor.1 {
                    let cursor_col = self.grid.cursor.0;
                    if cursor_col < width as usize {
                        execute!(
                            stdout(),
                            MoveTo(x + cursor_col as u16, screen_y),
                            SetBackgroundColor(Color::DarkBlue),
                            SetForegroundColor(Color::White),
                        )?;
                        
                        let ch = if grid_row < self.grid.cells.len() && cursor_col < self.grid.cells[grid_row].len() {
                            self.grid.cells[grid_row][cursor_col]
                        } else {
                            ' '
                        };
                        
                        execute!(stdout(), Print(ch), ResetColor)?;
                    }
                }
            }
        }
        
        // Render search box overlay if searching
        if self.grid.searching {
            // Draw search box in the middle of the text panel
            let box_width = 60.min(width - 4);
            let box_height = 3;
            let box_x = x + (width - box_width) / 2;
            let box_y = y + (height / 2).saturating_sub(1);
            
            // Draw box background
            execute!(stdout(), SetBackgroundColor(Color::DarkGrey))?;
            for row in 0..box_height {
                execute!(
                    stdout(),
                    MoveTo(box_x, box_y + row),
                    Print(" ".repeat(box_width as usize))
                )?;
            }
            
            // Draw border
            execute!(
                stdout(),
                SetForegroundColor(Color::Yellow),
                MoveTo(box_x, box_y),
                Print("╔"),
                Print("═".repeat((box_width - 2) as usize)),
                Print("╗"),
                MoveTo(box_x, box_y + 1),
                Print("║"),
                MoveTo(box_x + box_width - 1, box_y + 1),
                Print("║"),
                MoveTo(box_x, box_y + 2),
                Print("╚"),
                Print("═".repeat((box_width - 2) as usize)),
                Print("╝")
            )?;
            
            // Draw search prompt and query
            let search_text = format!("🔍 Search: {}_", self.grid.search_query);
            let text_x = box_x + 2;
            let text_y = box_y + 1;
            
            execute!(
                stdout(),
                MoveTo(text_x, text_y),
                SetBackgroundColor(Color::DarkGrey),
                SetForegroundColor(Color::White),
                Print(&search_text[..search_text.len().min((box_width - 4) as usize)]),
                ResetColor
            )?;
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
        self.image_sent = false; // Reset flag so new page image is sent
        self.viewports.pdf_viewport.mark_dirty();
        self.viewports.status_viewport.mark_dirty();
    }
    
    pub fn prev_page(&mut self) {
        if self.current_page > 1 {
            self.current_page -= 1;
            self.scroll_offset = 0;
            self.image_sent = false; // Reset flag so new page image is sent
            self.viewports.pdf_viewport.mark_dirty();
            self.viewports.status_viewport.mark_dirty();
        }
    }
    
    pub fn scroll_up(&mut self) {
        match self.current_screen {
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
        self.current_screen = screen;
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
    
    /// Handle keyboard input for Excel grid editing
    pub fn handle_grid_input(&mut self, key: crossterm::event::KeyCode, shift: bool) {
        // Only mark dirty for keys that actually change content
        let needs_redraw = match key {
            // These keys change content
            crossterm::event::KeyCode::Char(_) |
            crossterm::event::KeyCode::Delete |
            crossterm::event::KeyCode::Backspace |
            crossterm::event::KeyCode::Enter => true,
            
            // Arrow keys only need redraw if selecting (shift held)
            crossterm::event::KeyCode::Up |
            crossterm::event::KeyCode::Down |
            crossterm::event::KeyCode::Left |
            crossterm::event::KeyCode::Right => shift,
            
            // These don't change display
            _ => false,
        };
        
        self.grid.handle_key(key, shift);
        
        if needs_redraw {
            self.right_panel_dirty = true;
        }
    }
    
    /// Handle keyboard input with full modifiers for advanced editing
    pub fn handle_grid_input_with_modifiers(&mut self, key: crossterm::event::KeyCode, shift: bool, ctrl: bool, alt: bool) {
        // Only mark dirty for keys that actually change content
        let needs_redraw = match key {
            // Ctrl+V pastes content
            crossterm::event::KeyCode::Char('v') if ctrl => true,
            // Ctrl+X cuts content
            crossterm::event::KeyCode::Char('x') if ctrl => true,
            // Other ctrl+char combos might not change display
            crossterm::event::KeyCode::Char(_) if ctrl => false,
            // Regular chars change content
            crossterm::event::KeyCode::Char(_) => true,
            // Delete/Backspace change content
            crossterm::event::KeyCode::Delete |
            crossterm::event::KeyCode::Backspace |
            crossterm::event::KeyCode::Enter => true,
            // Arrow keys with shift (selection)
            crossterm::event::KeyCode::Up |
            crossterm::event::KeyCode::Down |
            crossterm::event::KeyCode::Left |
            crossterm::event::KeyCode::Right => shift,
            _ => false,
        };
        
        self.grid.handle_key_with_modifiers(key, shift, ctrl, alt);
        
        if needs_redraw {
            self.right_panel_dirty = true;
        }
    }
    
    /// Check if status message changed (for redraw detection)
    pub fn has_status_message(&self) -> bool {
        self.grid.get_status_message().is_some()
    }
    
    /// Check if Excel grid is in selection mode
    pub fn is_selecting(&self) -> bool {
        self.grid.selecting
    }
    
    /// Check if Excel grid is in search mode
    pub fn is_searching(&self) -> bool {
        self.grid.searching
    }
    
    /// Get Excel grid cursor position
    pub fn get_grid_cursor(&self) -> (usize, usize) {
        self.grid.cursor
    }
    
    /// Enhanced mouse handling with zone detection, scrolling, and zoom
    pub fn handle_mouse_enhanced(&mut self, event: crossterm::event::MouseEvent) -> bool {
        use crossterm::event::{MouseEventKind, MouseButton, KeyModifiers};
        
        // Update zone detection
        let zone = self.mouse_handler.detect_zone(event.column, event.row);
        let mut needs_redraw = false;
        
        // Handle scroll events
        match event.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                // Check for zoom (Ctrl+Scroll)
                if event.modifiers.contains(KeyModifiers::CONTROL) {
                    let (handled, action) = self.mouse_handler.handle_zoom(
                        &event, 
                        matches!(event.kind, MouseEventKind::ScrollUp)
                    );
                    if handled {
                        match action {
                            ZoomAction::PdfZoom(level) => {
                                self.pdf_zoom = level;
                                self.image_sent = false; // Re-render PDF at new zoom
                                self.viewports.pdf_viewport.mark_dirty();
                                needs_redraw = true;
                            }
                            ZoomAction::TextZoom(level) => {
                                self.text_zoom = level;
                                self.right_panel_dirty = true;
                                self.viewports.text_viewport.mark_dirty();
                                needs_redraw = true;
                            }
                            _ => {}
                        }
                    }
                } else {
                    // Regular scrolling (check for Shift for horizontal)
                    let horizontal = event.modifiers.contains(KeyModifiers::SHIFT);
                    let (handled, action) = self.mouse_handler.handle_scroll(&event, horizontal);
                    if handled {
                        match action {
                            ScrollAction::PdfScroll(_x, y) => {
                                // Update PDF scroll offset
                                self.scroll_offset = (y as usize).min(100); // Clamp to reasonable range
                                self.viewports.pdf_viewport.mark_dirty();
                                needs_redraw = true;
                            }
                            ScrollAction::TextScroll(_x, y) => {
                                // Update text scroll offset
                                self.grid.scroll_y = y;
                                self.right_panel_dirty = true;
                                self.viewports.text_viewport.mark_dirty();
                                needs_redraw = true;
                            }
                            _ => {}
                        }
                    }
                }
            }
            MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight => {
                // Horizontal scrolling
                let (handled, action) = self.mouse_handler.handle_scroll(&event, true);
                if handled {
                    match action {
                        ScrollAction::TextScroll(x, _y) => {
                            self.grid.scroll_x = x;
                            self.right_panel_dirty = true;
                            self.viewports.text_viewport.mark_dirty();
                            needs_redraw = true;
                        }
                        _ => {}
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left) | 
            MouseEventKind::Drag(MouseButton::Left) | 
            MouseEventKind::Up(MouseButton::Left) => {
                // Handle clicks/drags based on zone
                match zone {
                    MouseZone::TextEditor => {
                        // Pass to existing grid handler
                        self.handle_mouse_for_grid(event);
                        self.viewports.text_viewport.mark_dirty();
                        needs_redraw = true;
                    }
                    MouseZone::PdfPanel => {
                        // Could implement PDF interaction here (e.g., text selection)
                        needs_redraw = false;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        
        needs_redraw
    }
    
    /// Handle mouse events for Excel grid
    pub fn handle_mouse_for_grid(&mut self, event: crossterm::event::MouseEvent) {
        // Check if mouse is in the right panel (text area)
        let (term_width, _term_height) = match terminal::size() {
            Ok((w, h)) => (w, h),
            Err(_) => return,
        };
        
        let split_col = term_width / 2;
        let max_grid_width = (term_width - split_col - 4) as usize; // Available width for grid
        
        // Only handle if click is in the right panel
        if event.column >= split_col + 2 {  // +2 for border and padding
            let grid_col = (event.column - split_col - 2) as usize;
            let grid_row = event.row.saturating_sub(2) as usize;  // -2 for header
            
            // Clamp grid_col to the visible area width
            let grid_col = grid_col.min(max_grid_width.saturating_sub(1));
            
            use crossterm::event::MouseEventKind;
            match event.kind {
                MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                    self.grid.handle_mouse_down(grid_col, grid_row);
                    self.right_panel_dirty = true;  // Mark for redraw when selection starts
                }
                MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
                    self.grid.handle_mouse_drag(grid_col, grid_row);
                    self.right_panel_dirty = true;  // Mark for redraw during selection
                }
                MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
                    self.grid.handle_mouse_up(grid_col, grid_row);
                    self.right_panel_dirty = true;  // Mark for redraw when selection ends
                }
                _ => {}
            }
        }
    }
    
    /// Save the edited text to a file
    pub fn save_edited_text(&self, path: &PathBuf) -> Result<()> {
        let content = self.grid.to_string();
        std::fs::write(path, content)?;
        Ok(())
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
        }
    }
    
    pub fn load_pdf(&mut self, pdf_path: PathBuf) -> Result<()> {
        // Clear debug messages for new PDF load
        self.debug_messages.clear();
        self.debug_scroll_offset = 0;
        self.image_sent = false; // Reset flag for new PDF
        self.right_panel_dirty = true; // Mark right panel for redraw with new content
        
        // Mark all viewports as dirty for new PDF
        self.viewports.pdf_viewport.mark_dirty();
        self.viewports.text_viewport.mark_dirty();
        self.viewports.status_viewport.mark_dirty();
        
        // Clear extraction method to ensure proper detection for each PDF
        self.extraction_method = None;
        
        let msg = format!("A-B Comparison: Loading PDF {:?}", pdf_path);
        // eprintln!("[INFO] Left pane: lopdf-kitty rendering");
        // eprintln!("[INFO] Right pane: pdftotext extraction");
        self.add_debug_message(msg.clone());
        // eprintln!("[DEBUG] {}", msg);
        
        // Load PDF page count - chonker7 style with fresh instance
        self.add_debug_message("Getting page count...".to_string());
        // eprintln!("[DEBUG] Getting page count...");
        self.total_pages = pdf_renderer::get_pdf_page_count(&pdf_path)?;
        self.current_page = 1;
        let msg = format!("Page count: {}", self.total_pages);
        self.add_debug_message(msg.clone());
        // eprintln!("[DEBUG] {}", msg);
        
        // Render first page image - same size as chonker7
        self.add_debug_message("Rendering PDF with lopdf-kitty...".to_string());
        let image = pdf_renderer::render_pdf_page(&pdf_path, 0, 800, 1000)?;  // Same as chonker7
        
        // Dark mode is already applied in the PDF renderer - don't double-invert!
        // image = self.apply_dark_mode_filter(image);
        self.add_debug_message("PDF page rendered".to_string());
        
        // Skip ML analysis - just use pdftotext directly
        
        // Extract text using pdftotext for the right panel
        self.add_debug_message("Extracting text with pdftotext...".to_string());
        // eprintln!("[DEBUG] Running pdftotext with layout preservation...");
        
        let mut extraction_result = match std::process::Command::new("pdftotext")
            .args(&[
                "-layout",  // Preserve layout
                "-nopgbrk", // No page breaks
                "-f", "1",  // First page
                "-l", "1",  // Last page
                pdf_path.to_str().unwrap(),
                "-"  // Output to stdout
            ])
            .stderr(std::process::Stdio::null())  // Suppress stderr
            .output() {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                // eprintln!("[DEBUG] pdftotext extracted {} characters", text.len());
                text
            }
            _ => {
                // eprintln!("[WARNING] pdftotext failed, using fallback");
                "".to_string() // Empty string to trigger OCR check
            }
        };
        
        // Check if we need OCR (scanned PDF with no text layer)
        use crate::pdf::ocr;
        if ocr::needs_ocr(&extraction_result) {
            // eprintln!("[INFO] PDF appears to be scanned, attempting OCR...");
            self.add_debug_message("No text layer detected, attempting OCR...".to_string());
            
            // Mark viewports dirty when switching to OCR mode
            self.viewports.text_viewport.mark_dirty();
            self.viewports.status_viewport.mark_dirty();
            
            // Use the already-rendered PDF image for OCR
            if let Some(ref image) = self.current_pdf_image {
                match ocr::ocr_image(image) {
                    Ok(ocr_text) => {
                        // eprintln!("[DEBUG] OCR extracted {} characters", ocr_text.len());
                        extraction_result = ocr_text;
                        self.extraction_method = Some("OCR (Tesseract)".to_string());
                    }
                    Err(e) => {
                        // eprintln!("[ERROR] OCR failed: {}", e);
                        self.add_debug_message(format!("OCR failed: {}", e));
                        extraction_result = format!("OCR failed: {}\n\nThis PDF appears to be scanned and requires OCR.", e);
                        self.extraction_method = Some("Failed OCR".to_string());
                    }
                }
            } else {
                extraction_result = "No image available for OCR".to_string();
                self.extraction_method = Some("No Image".to_string());
            }
        } else {
            self.extraction_method = Some("pdftotext".to_string());
            
            // Mark viewports dirty when switching back to pdftotext mode
            if self.extraction_method != Some("pdftotext".to_string()) {
                self.viewports.text_viewport.mark_dirty();
                self.viewports.status_viewport.mark_dirty();
            }
        }
        
        let msg = format!("Extraction complete using {}", self.extraction_method.as_ref().unwrap_or(&"unknown".to_string()));
        self.add_debug_message(msg.clone());
        // eprintln!("[DEBUG] {}", msg);
        
        // Store metadata
        self.extraction_quality = Some(0.8);
        self.extraction_timestamp = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        
        // Just use the extracted text directly, no metadata box
        let text_with_metadata = extraction_result.clone();
        
        // Convert extracted text to grid format for display
        let text_matrix = self.text_to_matrix(&text_with_metadata, 200, 100);
        
        // Calculate actual available width for Excel grid based on terminal size
        let (term_width, term_height) = terminal::size().unwrap_or((80, 24));
        let grid_width = (term_width / 2 - 4) as usize; // Half terminal minus borders
        
        // Update Excel grid with the extracted text
        self.grid = Grid::from_pdftext(&text_with_metadata, grid_width.max(40));
        
        // Update state
        self.current_pdf_path = Some(pdf_path);
        self.current_pdf_image = Some(image);
        self.pdf_content = text_matrix;
        
        // Default to dark mode
        self.dark_mode = true;
        
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
            .stderr(std::process::Stdio::null())  // Suppress stderr
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