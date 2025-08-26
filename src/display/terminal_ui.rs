// Dynamic UI renderer that reads from hot-reloadable config
use crate::core::config::UIConfig;
use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Attribute, Attributes, Color, Print, ResetColor, SetAttributes, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{stdout, Write};
use std::path::PathBuf;
use image::DynamicImage;
use crate::display::file_browser::IntegratedFilePicker;
use crate::pdf::{page_renderer as pdf_renderer, extract_text as content_extractor};
use crate::display::kitty_graphics::KittyProtocol;
use crate::display::excel_grid::ExcelGrid;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    FilePicker,
    PdfViewer,
}

pub struct UIRenderer {
    config: UIConfig,
    pdf_content: Vec<Vec<char>>,
    excel_grid: ExcelGrid,  // Excel-style editable grid
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
            excel_grid: ExcelGrid::new(80, 50),  // Initialize Excel grid
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
        // Chonker7-style split view: PDF image on left, text extraction on right
        let (width, height) = terminal::size()?;
        let split_x = width / 2;
        
        // Only clear screen on first render to prevent flicker
        if self.first_render {
            execute!(
                stdout(),
                Clear(ClearType::All),
                MoveTo(0, 0)
            )?;
            self.first_render = false;
        } else {
            // Just move to home position, don't hide cursor here
            execute!(
                stdout(),
                MoveTo(0, 0)
            )?;
        }
        
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
            Print("◀ PDF RENDER (lopdf→kitty) ▶"),
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
            execute!(
                stdout(),
                MoveTo(2, 5),
                SetForegroundColor(Color::Red),
                Print("[ERROR: No PDF image loaded]")
            )?;
        }
        
        // Draw header for right panel
        execute!(
            stdout(),
            MoveTo(split_x + 2, 0),
            SetForegroundColor(Color::Green),
            SetAttributes(Attributes::from(Attribute::Bold)),
            Print("◀ TEXT EXTRACTION (editable) ▶"),
            SetAttributes(Attributes::from(Attribute::Reset))
        )?;
        
        // Show extraction metadata if available
        if let Some(method) = &self.extraction_method {
            execute!(
                stdout(),
                MoveTo(split_x + 2, 1),
                SetForegroundColor(Color::DarkGrey),
                Print(format!("Method: {} | Quality: {:.0}%", 
                    method,
                    self.extraction_quality.unwrap_or(0.0) * 100.0
                )),
                ResetColor
            )?;
        }
        
        // Render text extraction on right side using the Excel grid
        self.render_text_content(split_x + 2, 2, width - split_x - 4, height - 4)?;
        
        // Status bar with Excel grid status
        let mut status_text = if let Some(path) = &self.current_pdf_path {
            format!("PDF: {} | Page: {}/{}", 
                path.file_name().unwrap_or_default().to_string_lossy(),
                self.current_page, 
                self.total_pages)
        } else {
            "PDF - TEST Screen".to_string()
        };
        
        // Add Excel grid status message if available
        if let Some(msg) = self.excel_grid.get_status_message() {
            status_text.push_str(&format!(" | {}", msg));
        } else {
            // Show basic shortcuts only
            status_text.push_str(" | ^C:Copy ^X:Cut ^V:Paste");
        }
        
        status_text.push_str(" | Tab:Switch • Esc:Exit");
        
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
    
    /*
    // Debug screen removed - keeping skeleton for potential future use
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
        // ALWAYS use Kitty protocol - NO FALLBACK
        if let Some(ref image) = self.current_pdf_image {
            // Always send the image - Kitty protocol handles replacing existing images
            // The clear command with same ID will replace the old one
            self.image_sent = true;
            
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
            
            // Calculate scale to fit within the left panel (half the terminal width)
            // The panel dimensions are in terminal cells, not pixels
            // Kitty graphics use cells as units for placement
            let panel_width_cells = width.saturating_sub(4);  // Leave some padding
            let panel_height_cells = height.saturating_sub(4);
            
            // For Kitty protocol, we specify size in terminal cells
            // The image will be scaled to fit within these dimensions
            let display_width = panel_width_cells.min(50) as u32;  // Cap at reasonable size
            let display_height = panel_height_cells.min(35) as u32;
            
            // Position at top-left of the panel with small margin
            let image_x = x + 2;
            let image_y = y + 2;
            
            // Move cursor to position
            execute!(
                stdout(),
                MoveTo(image_x, image_y)
            )?;
            
            // Send image at fixed position within panel
            match KittyImage::send_image_positioned(image, display_width, display_height, image_x, image_y) {
                Ok(_) => {
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
        // Render the Excel grid with block selection
        for row in 0..height.min(self.excel_grid.cells.len() as u16) {
            execute!(stdout(), MoveTo(x, y + row))?;
            
            // Line numbers
            if self.config.panels.text.line_numbers && width > 5 {
                execute!(
                    stdout(),
                    SetForegroundColor(Color::DarkGrey),
                    Print(format!("{:4}│", row + 1)),  // Line numbers start at 1
                    ResetColor,
                )?;
                
                // Grid content
                let text_start = 5;
                let text_width = width - text_start;
                
                for col in 0..text_width.min(self.excel_grid.width as u16) {
                    let grid_row = row as usize;  // Don't add scroll_offset - excel grid handles its own coordinates
                    let grid_col = col as usize;
                    
                    let is_cursor = self.excel_grid.cursor == (grid_col, grid_row);
                    let is_selected = self.excel_grid.is_selected(grid_col, grid_row);
                    
                    
                    // Apply colors for selection and cursor
                    if is_cursor && !self.excel_grid.selecting {
                        // Bright yellow cursor when not selecting
                        execute!(
                            stdout(),
                            SetBackgroundColor(Color::Yellow),
                            SetForegroundColor(Color::Black),
                        )?;
                    } else if is_selected {
                        // Blue background for selected text
                        execute!(
                            stdout(),
                            SetBackgroundColor(Color::Blue),
                            SetForegroundColor(Color::White),
                        )?;
                    } else if is_cursor {
                        // Cyan cursor when selecting
                        execute!(
                            stdout(),
                            SetBackgroundColor(Color::Cyan),
                            SetForegroundColor(Color::Black),
                        )?;
                    } else {
                        execute!(stdout(), SetForegroundColor(self.config.get_text_color()))?;
                    }
                    
                    // Print character
                    let ch = if grid_row < self.excel_grid.cells.len() && grid_col < self.excel_grid.cells[grid_row].len() {
                        self.excel_grid.cells[grid_row][grid_col]
                    } else {
                        ' '
                    };
                    
                    execute!(stdout(), Print(ch))?;
                    
                    if is_cursor || is_selected {
                        execute!(stdout(), ResetColor)?;
                    }
                }
            } else {
                // No line numbers - render grid directly
                for col in 0..width.min(self.excel_grid.width as u16) {
                    let grid_row = row as usize;  // Don't add scroll_offset - excel grid handles its own coordinates
                    let grid_col = col as usize;
                    
                    let is_cursor = self.excel_grid.cursor == (grid_col, grid_row);
                    let is_selected = self.excel_grid.is_selected(grid_col, grid_row);
                    
                    if is_cursor && !self.excel_grid.selecting {
                        // Bright yellow cursor when not selecting
                        execute!(
                            stdout(),
                            SetBackgroundColor(Color::Yellow),
                            SetForegroundColor(Color::Black),
                        )?;
                    } else if is_selected {
                        // Blue background for selected text
                        execute!(
                            stdout(),
                            SetBackgroundColor(Color::Blue),
                            SetForegroundColor(Color::White),
                        )?;
                    } else if is_cursor {
                        // Cyan cursor when selecting
                        execute!(
                            stdout(),
                            SetBackgroundColor(Color::Cyan),
                            SetForegroundColor(Color::Black),
                        )?;
                    } else {
                        execute!(stdout(), SetForegroundColor(self.config.get_text_color()))?;
                    }
                    
                    let ch = if grid_row < self.excel_grid.cells.len() && grid_col < self.excel_grid.cells[grid_row].len() {
                        self.excel_grid.cells[grid_row][grid_col]
                    } else {
                        ' '
                    };
                    
                    execute!(stdout(), Print(ch))?;
                    
                    if is_cursor || is_selected {
                        execute!(stdout(), ResetColor)?;
                    }
                }
            }
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
    }
    
    pub fn prev_page(&mut self) {
        if self.current_page > 1 {
            self.current_page -= 1;
            self.scroll_offset = 0;
            self.image_sent = false; // Reset flag so new page image is sent
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
    pub fn handle_excel_grid_input(&mut self, key: crossterm::event::KeyCode, shift: bool) {
        self.excel_grid.handle_key(key, shift);
    }
    
    /// Handle keyboard input with full modifiers for advanced editing
    pub fn handle_excel_grid_input_with_modifiers(&mut self, key: crossterm::event::KeyCode, shift: bool, ctrl: bool, alt: bool) {
        self.excel_grid.handle_key_with_modifiers(key, shift, ctrl, alt);
    }
    
    /// Check if status message changed (for redraw detection)
    pub fn has_status_message(&self) -> bool {
        self.excel_grid.get_status_message().is_some()
    }
    
    /// Check if Excel grid is in selection mode
    pub fn is_selecting(&self) -> bool {
        self.excel_grid.selecting
    }
    
    /// Get Excel grid cursor position
    pub fn get_grid_cursor(&self) -> (usize, usize) {
        self.excel_grid.cursor
    }
    
    /// Handle mouse events for Excel grid
    pub fn handle_mouse_for_excel_grid(&mut self, event: crossterm::event::MouseEvent) {
        // Check if mouse is in the right panel (text area)
        let (term_width, _term_height) = match terminal::size() {
            Ok((w, h)) => (w, h),
            Err(_) => return,
        };
        
        let split_col = term_width / 2;
        
        // Only handle if click is in the right panel
        if event.column >= split_col + 2 {  // +2 for border and padding
            let grid_col = (event.column - split_col - 2) as usize;
            let grid_row = event.row.saturating_sub(2) as usize;  // -2 for header
            
            use crossterm::event::MouseEventKind;
            match event.kind {
                MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                    self.excel_grid.handle_mouse_down(grid_col, grid_row);
                }
                MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
                    self.excel_grid.handle_mouse_drag(grid_col, grid_row);
                }
                MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
                    self.excel_grid.handle_mouse_up(grid_col, grid_row);
                }
                _ => {}
            }
        }
    }
    
    /// Save the edited text to a file
    pub fn save_edited_text(&self, path: &PathBuf) -> Result<()> {
        let content = self.excel_grid.to_string();
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
        use crate::ml_extraction::{DocumentAnalyzer, PageFingerprint};
        
        // Clear debug messages for new PDF load
        self.debug_messages.clear();
        self.debug_scroll_offset = 0;
        self.image_sent = false; // Reset flag for new PDF
        
        let msg = format!("A-B Comparison: Loading PDF {:?}", pdf_path);
        eprintln!("[INFO] Left pane: lopdf-kitty rendering");
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
        
        // Render first page image - same size as chonker7
        self.add_debug_message("Rendering PDF with lopdf-kitty...".to_string());
        let mut image = pdf_renderer::render_pdf_page(&pdf_path, 0, 800, 1000)?;  // Same as chonker7
        
        // Apply dark mode filter for better visibility
        image = self.apply_dark_mode_filter(image);
        self.add_debug_message("PDF page rendered".to_string());
        
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
                crate::ml_extraction::ExtractionResult {
                    text,
                    quality_score: 0.8,
                    method: crate::ml_extraction::ExtractionMethod::PdfToText,
                    extraction_time_ms: 0,
                }
            }
            _ => {
                eprintln!("[WARNING] pdftotext failed, using fallback");
                crate::ml_extraction::ExtractionResult {
                    text: "Text extraction failed - pdftotext not available".to_string(),
                    quality_score: 0.0,
                    method: crate::ml_extraction::ExtractionMethod::PdfToText,
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
        
        // Update Excel grid with the extracted text
        self.excel_grid = ExcelGrid::from_pdftext(&text_with_metadata, 80);
        
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