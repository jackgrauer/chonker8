// Enhanced A-B Comparison UI with Excel Grid for text editing
// Left: PDF image via Kitty protocol
// Right: Editable Excel-style grid with block selection

use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    event::KeyCode,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor, Attribute, SetAttribute},
    terminal::{self, Clear, ClearType},
};
use std::io::{stdout, Write};
use std::path::PathBuf;
use image::DynamicImage;
use crate::display::kitty_graphics::KittyProtocol;
use crate::display::terminal_ui::ExcelGrid;

pub struct ABComparisonExcel {
    // PDF side
    pdf_image: Option<DynamicImage>,
    current_page: usize,
    total_pages: usize,
    pdf_path: Option<PathBuf>,
    kitty_protocol: KittyProtocol,
    pdf_image_id: Option<u32>,
    
    // Excel grid side
    excel_grid: ExcelGrid,
    
    // UI state
    focus: Focus,
    status_message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    PdfView,
    ExcelGrid,
}

impl ABComparisonExcel {
    pub fn new() -> Self {
        let mut kitty = KittyProtocol::new();
        kitty.force_enable(); // For testing
        
        Self {
            pdf_image: None,
            current_page: 1,
            total_pages: 1,
            pdf_path: None,
            kitty_protocol: kitty,
            pdf_image_id: None,
            excel_grid: ExcelGrid::new(80, 50),
            focus: Focus::ExcelGrid,
            status_message: "Ready - Tab: Switch Focus | ^V: Block Select | ^Q: Quit".to_string(),
        }
    }
    
    /// Load PDF content - image and extracted text
    pub fn load_pdf(&mut self, image: DynamicImage, text: &str, page: usize, total: usize) {
        self.pdf_image = Some(image);
        self.current_page = page;
        self.total_pages = total;
        
        // Load text into Excel grid
        self.excel_grid = ExcelGrid::from_pdftext(text, 80);
        self.status_message = format!("Loaded page {}/{}", page, total);
    }
    
    /// Main render function
    pub fn render(&mut self) -> Result<()> {
        let (width, height) = terminal::size()?;
        
        // Clear screen
        execute!(
            stdout(),
            Clear(ClearType::All),
            MoveTo(0, 0),
            Hide,
        )?;
        
        // Calculate split
        let split_x = width / 2;
        
        // Render both panels
        self.render_pdf_panel(0, 0, split_x, height - 2)?;
        self.render_excel_panel(split_x + 1, 0, width - split_x - 1, height - 2)?;
        
        // Render divider
        self.render_divider(split_x, height - 2)?;
        
        // Status bar
        self.render_status_bar(width, height)?;
        
        stdout().flush()?;
        Ok(())
    }
    
    fn render_pdf_panel(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        // Header
        let header_color = if self.focus == Focus::PdfView {
            Color::Rgb { r: 100, g: 200, b: 255 }
        } else {
            Color::Rgb { r: 80, g: 80, b: 100 }
        };
        
        execute!(
            stdout(),
            MoveTo(x, y),
            SetForegroundColor(header_color),
            SetAttribute(Attribute::Bold),
            Print(format!("┌─ 📄 PDF Page {}/{} ", self.current_page, self.total_pages)),
            Print("─".repeat((width as usize).saturating_sub(25))),
            Print("┐"),
            ResetColor,
        )?;
        
        // Content area
        for row in 1..height {
            execute!(
                stdout(),
                MoveTo(x, y + row),
                SetForegroundColor(header_color),
                Print("│"),
                ResetColor,
                Print(" ".repeat((width - 2) as usize)),
                SetForegroundColor(header_color),
                Print("│"),
                ResetColor,
            )?;
        }
        
        // Display PDF via Kitty protocol if available
        if let Some(ref pdf_image) = self.pdf_image {
            let img_x = (x + 2) as u32;
            let img_y = (y + 2) as u32;
            let img_width = Some((width - 4) as u32);
            let img_height = Some((height - 3) as u32);
            
            match self.kitty_protocol.display_image(pdf_image, img_x, img_y, img_width, img_height) {
                Ok(id) => {
                    self.pdf_image_id = Some(id);
                }
                Err(_) => {
                    execute!(
                        stdout(),
                        MoveTo(x + width/4, y + height/2),
                        SetForegroundColor(Color::DarkGrey),
                        Print("[PDF Image - Kitty Protocol Required]"),
                        ResetColor,
                    )?;
                }
            }
        } else {
            execute!(
                stdout(),
                MoveTo(x + width/4, y + height/2),
                SetForegroundColor(Color::DarkGrey),
                Print("No PDF loaded"),
                ResetColor,
            )?;
        }
        
        // Bottom border
        execute!(
            stdout(),
            MoveTo(x, y + height - 1),
            SetForegroundColor(header_color),
            Print("└"),
            Print("─".repeat((width - 2) as usize)),
            Print("┘"),
            ResetColor,
        )?;
        
        Ok(())
    }
    
    fn render_excel_panel(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        // Header
        let header_color = if self.focus == Focus::ExcelGrid {
            Color::Rgb { r: 100, g: 255, b: 150 }
        } else {
            Color::Rgb { r: 80, g: 100, b: 80 }
        };
        
        let mode_str = if self.excel_grid.selecting {
            "BLOCK SELECT"
        } else {
            "EDIT"
        };
        
        execute!(
            stdout(),
            MoveTo(x, y),
            SetForegroundColor(header_color),
            SetAttribute(Attribute::Bold),
            Print(format!("┌─ ✏️  {} - Excel Grid ", mode_str)),
            Print("─".repeat((width as usize).saturating_sub(30))),
            Print("┐"),
            ResetColor,
        )?;
        
        // Render grid content
        let content_height = (height - 2) as usize;
        let content_width = (width - 2) as usize;
        
        for row in 0..content_height.min(self.excel_grid.cells.len()) {
            execute!(
                stdout(),
                MoveTo(x, y + row as u16 + 1),
                SetForegroundColor(header_color),
                Print("│"),
                ResetColor,
            )?;
            
            // Line numbers
            if content_width > 5 {
                execute!(
                    stdout(),
                    SetForegroundColor(Color::DarkGrey),
                    Print(format!("{:4}│", row + 1)),
                    ResetColor,
                )?;
                
                // Grid content
                let text_start = 5;
                let text_width = content_width - text_start - 1;
                
                for col in 0..text_width.min(self.excel_grid.width) {
                    let is_cursor = self.excel_grid.cursor == (col, row);
                    let is_selected = self.excel_grid.is_selected(col, row);
                    
                    // Apply colors
                    if is_cursor && self.focus == Focus::ExcelGrid {
                        execute!(
                            stdout(),
                            SetBackgroundColor(Color::Rgb { r: 255, g: 255, b: 100 }),
                            SetForegroundColor(Color::Black),
                        )?;
                    } else if is_selected {
                        execute!(
                            stdout(),
                            SetBackgroundColor(Color::Rgb { r: 40, g: 60, b: 120 }),
                            SetForegroundColor(Color::White),
                        )?;
                    }
                    
                    // Print character
                    let ch = if row < self.excel_grid.cells.len() && col < self.excel_grid.cells[row].len() {
                        self.excel_grid.cells[row][col]
                    } else {
                        ' '
                    };
                    
                    execute!(stdout(), Print(ch))?;
                    
                    if is_cursor || is_selected {
                        execute!(stdout(), ResetColor)?;
                    }
                }
            }
            
            // End of line
            execute!(
                stdout(),
                SetForegroundColor(header_color),
                MoveTo(x + width - 1, y + row as u16 + 1),
                Print("│"),
                ResetColor,
            )?;
        }
        
        // Fill remaining rows
        for row in content_height.min(self.excel_grid.cells.len())..content_height {
            execute!(
                stdout(),
                MoveTo(x, y + row as u16 + 1),
                SetForegroundColor(header_color),
                Print("│"),
                Print(" ".repeat((width - 2) as usize)),
                MoveTo(x + width - 1, y + row as u16 + 1),
                Print("│"),
                ResetColor,
            )?;
        }
        
        // Bottom border
        execute!(
            stdout(),
            MoveTo(x, y + height - 1),
            SetForegroundColor(header_color),
            Print("└"),
            Print("─".repeat((width - 2) as usize)),
            Print("┘"),
            ResetColor,
        )?;
        
        // Show cursor in Excel grid if focused
        if self.focus == Focus::ExcelGrid {
            let cursor_x = x + 6 + self.excel_grid.cursor.0 as u16;
            let cursor_y = y + 1 + self.excel_grid.cursor.1 as u16;
            
            if cursor_x < x + width - 1 && cursor_y < y + height - 1 {
                execute!(stdout(), MoveTo(cursor_x, cursor_y), Show)?;
            }
        }
        
        Ok(())
    }
    
    fn render_divider(&self, x: u16, height: u16) -> Result<()> {
        execute!(
            stdout(),
            SetForegroundColor(Color::Rgb { r: 60, g: 60, b: 80 })
        )?;
        
        for y in 0..height {
            execute!(
                stdout(),
                MoveTo(x, y),
                Print("║")
            )?;
        }
        
        execute!(stdout(), ResetColor)?;
        Ok(())
    }
    
    fn render_status_bar(&self, width: u16, height: u16) -> Result<()> {
        let focus_str = match self.focus {
            Focus::PdfView => "[PDF VIEW]",
            Focus::ExcelGrid => "[EXCEL GRID]",
        };
        
        let pos_str = format!(
            " {}:{} ",
            self.excel_grid.cursor.1 + 1,
            self.excel_grid.cursor.0 + 1
        );
        
        let selection_str = if self.excel_grid.selecting {
            let (x1, y1, x2, y2) = self.excel_grid.get_selection_bounds();
            format!(" [{}×{}] ", x2 - x1 + 1, y2 - y1 + 1)
        } else {
            String::new()
        };
        
        execute!(
            stdout(),
            MoveTo(0, height - 1),
            SetBackgroundColor(Color::Rgb { r: 30, g: 30, b: 40 }),
            SetForegroundColor(Color::Rgb { r: 200, g: 200, b: 220 }),
            Print(format!(
                "{} {} {}{}",
                focus_str,
                pos_str,
                selection_str,
                " ".repeat((width as usize).saturating_sub(focus_str.len() + pos_str.len() + selection_str.len() + self.status_message.len()))
            )),
            Print(&self.status_message),
            ResetColor,
        )?;
        
        Ok(())
    }
    
    /// Handle keyboard input
    pub fn handle_key(&mut self, code: KeyCode, shift: bool) -> Result<()> {
        match code {
            KeyCode::Tab => {
                // Switch focus between panels
                self.focus = match self.focus {
                    Focus::PdfView => Focus::ExcelGrid,
                    Focus::ExcelGrid => Focus::PdfView,
                };
                self.status_message = format!("Focus: {:?}", self.focus);
            }
            _ if self.focus == Focus::ExcelGrid => {
                // Pass to Excel grid
                self.excel_grid.handle_key(code, shift);
                
                // Update status
                if self.excel_grid.selecting {
                    let (x1, y1, x2, y2) = self.excel_grid.get_selection_bounds();
                    self.status_message = format!(
                        "Selecting {}×{} block",
                        x2 - x1 + 1,
                        y2 - y1 + 1
                    );
                } else {
                    self.status_message = "Editing - ^V: Block Select".to_string();
                }
            }
            _ => {
                // PDF navigation when focused on PDF
                match code {
                    KeyCode::PageUp => {
                        self.status_message = "Previous page (not implemented)".to_string();
                    }
                    KeyCode::PageDown => {
                        self.status_message = "Next page (not implemented)".to_string();
                    }
                    _ => {}
                }
            }
        }
        
        Ok(())
    }
    
    /// Get the edited text
    pub fn get_text(&self) -> String {
        self.excel_grid.to_string()
    }
    
    /// Save the edited text
    pub fn save_text(&self, path: &PathBuf) -> Result<()> {
        std::fs::write(path, self.get_text())?;
        Ok(())
    }
}