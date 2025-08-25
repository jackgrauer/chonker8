// Excel Grid Demo - Beautiful Kitty terminal rendering
use anyhow::Result;
use chonker8::display::excel_grid::ExcelGrid;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor, Attribute, SetAttribute},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{stdout, Write};
use std::time::Duration;

struct ExcelGridApp {
    grid: ExcelGrid,
    status_message: String,
    show_help: bool,
    viewport_offset: (usize, usize), // For scrolling large documents
}

impl ExcelGridApp {
    fn new() -> Self {
        // Load with sample PDF text
        let sample_text = r#"INVOICE #2024-001                                              Date: 2024-03-15
================================================================================
                                                                               
Bill To:                                        Ship To:                       
Acme Corporation                                Same as Bill To                
123 Business Ave                                                               
Suite 500                                                                       
New York, NY 10001                                                            
                                                                               
--------------------------------------------------------------------------------
Item Description                    Qty    Unit Price    Amount                
--------------------------------------------------------------------------------
Professional Services                40     $150.00      $6,000.00             
Software License (Annual)            1      $2,500.00    $2,500.00             
Training & Support                   8      $200.00      $1,600.00             
                                                                               
                                           Subtotal:     $10,100.00            
                                           Tax (8%):     $808.00                
                                           TOTAL:        $10,908.00            
================================================================================
                                                                               
Terms: Net 30                          Thank you for your business!            "#;
        
        let mut grid = ExcelGrid::from_pdftext(sample_text, 80);
        
        Self {
            grid,
            status_message: "Ready - Press F1 for help".to_string(),
            show_help: false,
            viewport_offset: (0, 0),
        }
    }
    
    fn render(&self) -> Result<()> {
        let (term_width, term_height) = terminal::size()?;
        
        // Clear and prepare screen
        execute!(
            stdout(),
            Clear(ClearType::All),
            MoveTo(0, 0),
            Hide,
        )?;
        
        // Draw top border with title
        execute!(
            stdout(),
            SetForegroundColor(Color::Rgb { r: 100, g: 150, b: 255 }),
            SetAttribute(Attribute::Bold),
            Print("┌─ Excel Grid Editor "),
            Print("─".repeat((term_width as usize).saturating_sub(25))),
            Print("┐"),
            ResetColor,
        )?;
        
        // Calculate visible area
        let visible_height = (term_height - 4) as usize; // Leave room for borders and status
        let visible_width = (term_width - 2) as usize;
        
        // Render grid content
        for row in 0..visible_height.min(self.grid.cells.len()) {
            execute!(stdout(), MoveTo(0, row as u16 + 1))?;
            execute!(
                stdout(),
                SetForegroundColor(Color::Rgb { r: 100, g: 150, b: 255 }),
                Print("│"),
                ResetColor,
            )?;
            
            let grid_row = row + self.viewport_offset.1;
            if grid_row < self.grid.cells.len() {
                for col in 0..visible_width.min(self.grid.width) {
                    let grid_col = col + self.viewport_offset.0;
                    
                    // Check if this cell is selected
                    let is_selected = self.grid.is_selected(grid_col, grid_row);
                    let is_cursor = self.grid.cursor == (grid_col, grid_row);
                    
                    // Apply styling
                    if is_cursor && !self.grid.selecting {
                        // Cursor position - bright inverse
                        execute!(
                            stdout(),
                            SetBackgroundColor(Color::Rgb { r: 255, g: 255, b: 100 }),
                            SetForegroundColor(Color::Black),
                        )?;
                    } else if is_selected {
                        // Selected block - blue background
                        execute!(
                            stdout(),
                            SetBackgroundColor(Color::Rgb { r: 40, g: 60, b: 120 }),
                            SetForegroundColor(Color::White),
                        )?;
                    } else if is_cursor {
                        // Cursor in selection mode
                        execute!(
                            stdout(),
                            SetBackgroundColor(Color::Rgb { r: 60, g: 90, b: 180 }),
                            SetForegroundColor(Color::White),
                        )?;
                    }
                    
                    // Print character
                    let ch = if grid_col < self.grid.cells[grid_row].len() {
                        self.grid.cells[grid_row][grid_col]
                    } else {
                        ' '
                    };
                    
                    execute!(stdout(), Print(ch))?;
                    
                    // Reset colors after each character
                    if is_cursor || is_selected {
                        execute!(stdout(), ResetColor)?;
                    }
                }
                
                // Fill rest of line with spaces
                let remaining = visible_width.saturating_sub(self.grid.width);
                if remaining > 0 {
                    execute!(stdout(), Print(" ".repeat(remaining)))?;
                }
            } else {
                // Empty row
                execute!(stdout(), Print(" ".repeat(visible_width)))?;
            }
            
            execute!(
                stdout(),
                SetForegroundColor(Color::Rgb { r: 100, g: 150, b: 255 }),
                Print("│"),
                ResetColor,
            )?;
        }
        
        // Draw bottom border
        execute!(
            stdout(),
            MoveTo(0, term_height - 2),
            SetForegroundColor(Color::Rgb { r: 100, g: 150, b: 255 }),
            Print("└"),
            Print("─".repeat((term_width - 2) as usize)),
            Print("┘"),
            ResetColor,
        )?;
        
        // Status bar
        self.render_status_bar(term_width, term_height)?;
        
        // Show help overlay if requested
        if self.show_help {
            self.render_help_overlay(term_width, term_height)?;
        }
        
        // Position hardware cursor
        let screen_x = (self.grid.cursor.0 - self.viewport_offset.0 + 1) as u16;
        let screen_y = (self.grid.cursor.1 - self.viewport_offset.1 + 1) as u16;
        
        if screen_x < term_width - 1 && screen_y < term_height - 2 {
            execute!(
                stdout(),
                MoveTo(screen_x, screen_y),
                Show,
            )?;
        }
        
        stdout().flush()?;
        Ok(())
    }
    
    fn render_status_bar(&self, width: u16, height: u16) -> Result<()> {
        let pos_info = format!(
            " {}:{} ", 
            self.grid.cursor.1 + 1,
            self.grid.cursor.0 + 1
        );
        
        let mode = if self.grid.selecting {
            " [BLOCK SELECT] "
        } else {
            " [EDIT] "
        };
        
        let selection_info = if self.grid.selecting {
            let (x1, y1, x2, y2) = self.grid.get_selection_bounds();
            format!(" {}×{} ", x2 - x1 + 1, y2 - y1 + 1)
        } else {
            String::new()
        };
        
        execute!(
            stdout(),
            MoveTo(0, height - 1),
            SetBackgroundColor(Color::Rgb { r: 30, g: 30, b: 40 }),
            SetForegroundColor(Color::Rgb { r: 200, g: 200, b: 220 }),
        )?;
        
        // Left side
        execute!(
            stdout(),
            SetAttribute(Attribute::Bold),
            Print(pos_info),
            SetForegroundColor(if self.grid.selecting {
                Color::Rgb { r: 100, g: 200, b: 255 }
            } else {
                Color::Rgb { r: 100, g: 255, b: 150 }
            }),
            Print(mode),
            SetForegroundColor(Color::Rgb { r: 255, g: 200, b: 100 }),
            Print(selection_info),
            ResetColor,
            SetBackgroundColor(Color::Rgb { r: 30, g: 30, b: 40 }),
        )?;
        
        // Center message
        let center_start = (width / 2) - (self.status_message.len() as u16 / 2);
        execute!(
            stdout(),
            MoveTo(center_start, height - 1),
            SetForegroundColor(Color::Rgb { r: 150, g: 150, b: 180 }),
            Print(&self.status_message),
        )?;
        
        // Right side
        let help_text = " F1:Help  ^V:Block  ^Q:Quit ";
        execute!(
            stdout(),
            MoveTo(width - help_text.len() as u16, height - 1),
            SetForegroundColor(Color::Rgb { r: 120, g: 120, b: 150 }),
            Print(help_text),
        )?;
        
        // Fill remaining space
        execute!(
            stdout(),
            SetBackgroundColor(Color::Rgb { r: 30, g: 30, b: 40 }),
            Print(" ".repeat(width as usize)),
            ResetColor,
        )?;
        
        Ok(())
    }
    
    fn render_help_overlay(&self, width: u16, height: u16) -> Result<()> {
        let help_lines = vec![
            "╔════════════════════════════════════════╗",
            "║         EXCEL GRID SHORTCUTS           ║",
            "╠════════════════════════════════════════╣",
            "║ Navigation:                            ║",
            "║   ↑↓←→     Move cursor                 ║",
            "║   Home/End  Line start/end             ║",
            "║   PgUp/PgDn Scroll 10 lines            ║",
            "║                                        ║",
            "║ Selection:                             ║",
            "║   Ctrl+V    Toggle block selection     ║",
            "║   Shift+↑↓←→ Start selection & move    ║",
            "║   Escape    Cancel selection           ║",
            "║                                        ║",
            "║ Editing:                               ║",
            "║   Type      Insert/replace text        ║",
            "║   Delete    Clear cell/selection       ║",
            "║   Backspace Delete backwards           ║",
            "║                                        ║",
            "║ Press F1 to close help                 ║",
            "╚════════════════════════════════════════╝",
        ];
        
        let box_width = 44;
        let box_height = help_lines.len();
        let start_x = (width / 2) - (box_width as u16 / 2);
        let start_y = (height / 2) - (box_height as u16 / 2);
        
        for (i, line) in help_lines.iter().enumerate() {
            execute!(
                stdout(),
                MoveTo(start_x, start_y + i as u16),
                SetBackgroundColor(Color::Rgb { r: 20, g: 20, b: 30 }),
                SetForegroundColor(Color::Rgb { r: 200, g: 200, b: 255 }),
                Print(line),
            )?;
        }
        
        execute!(stdout(), ResetColor)?;
        Ok(())
    }
    
    fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(true); // Quit
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.grid.handle_key(KeyCode::Char('v'), false);
                self.status_message = if self.grid.selecting {
                    "Block selection mode - Move cursor to select".to_string()
                } else {
                    "Selection cancelled".to_string()
                };
            }
            KeyCode::F(1) => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.status_message = "Document saved (simulation)".to_string();
            }
            _ => {
                let shift_held = key.modifiers.contains(KeyModifiers::SHIFT);
                self.grid.handle_key(key.code, shift_held);
                
                // Update status based on action
                if self.grid.selecting && shift_held {
                    let (x1, y1, x2, y2) = self.grid.get_selection_bounds();
                    self.status_message = format!(
                        "Selecting {}×{} block",
                        x2 - x1 + 1,
                        y2 - y1 + 1
                    );
                }
            }
        }
        
        // Update viewport to follow cursor
        self.update_viewport();
        
        Ok(false)
    }
    
    fn update_viewport(&mut self) {
        let (term_width, term_height) = terminal::size().unwrap_or((80, 24));
        let visible_height = (term_height - 4) as usize;
        let visible_width = (term_width - 2) as usize;
        
        // Vertical scrolling
        if self.grid.cursor.1 < self.viewport_offset.1 {
            self.viewport_offset.1 = self.grid.cursor.1;
        } else if self.grid.cursor.1 >= self.viewport_offset.1 + visible_height {
            self.viewport_offset.1 = self.grid.cursor.1 - visible_height + 1;
        }
        
        // Horizontal scrolling
        if self.grid.cursor.0 < self.viewport_offset.0 {
            self.viewport_offset.0 = self.grid.cursor.0;
        } else if self.grid.cursor.0 >= self.viewport_offset.0 + visible_width {
            self.viewport_offset.0 = self.grid.cursor.0 - visible_width + 1;
        }
    }
}

fn main() -> Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, Hide)?;
    
    let mut app = ExcelGridApp::new();
    
    // Main loop
    loop {
        app.render()?;
        
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if app.handle_key(key)? {
                    break;
                }
            }
        }
    }
    
    // Cleanup
    execute!(stdout(), Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    
    Ok(())
}