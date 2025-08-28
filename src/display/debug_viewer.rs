use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{stdout, Write};
use arboard::Clipboard;

pub enum DebugViewerExit {
    Tab,    // User pressed Tab to cycle
    Escape, // User pressed Esc to exit app
}

pub struct DebugViewer {
    content: Vec<String>,
    scroll_offset: usize,
    viewport_height: usize,
    viewport_width: usize,
    
    // Text selection
    selection_start: Option<(u16, u16)>, // (x, y) in screen coordinates
    selection_end: Option<(u16, u16)>,
    is_selecting: bool,
    
    // Mouse tracking
    last_mouse_pos: (u16, u16),
}

impl DebugViewer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            content: Vec::new(),
            scroll_offset: 0,
            viewport_height: height.saturating_sub(4), // Leave room for header/footer
            viewport_width: width.saturating_sub(4),   // Leave margins
            selection_start: None,
            selection_end: None,
            is_selecting: false,
            last_mouse_pos: (0, 0),
        }
    }
    
    pub fn load_from_file(&mut self, path: &str) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.content = content.lines().map(|s| s.to_string()).collect();
        self.scroll_offset = 0;
        Ok(())
    }
    
    pub fn load_from_string(&mut self, content: String) {
        self.content = content.lines().map(|s| s.to_string()).collect();
        self.scroll_offset = 0;
    }
    
    pub fn render(&self) -> Result<()> {
        let mut stdout = stdout();
        
        // Clear screen
        queue!(stdout, Clear(ClearType::All))?;
        
        // Draw header
        queue!(
            stdout,
            MoveTo(0, 0),
            SetForegroundColor(Color::Cyan),
            Print("═".repeat(self.viewport_width + 4)),
            MoveTo(2, 1),
            SetForegroundColor(Color::Yellow),
            Print("Debug Viewer - Mouse: Select | Ctrl+C: Copy | Tab: Next | Esc: Exit"),
            ResetColor,
        )?;
        
        // Draw content with potential selection highlighting
        let start_line = self.scroll_offset;
        let end_line = (self.scroll_offset + self.viewport_height).min(self.content.len());
        
        for (idx, line_num) in (start_line..end_line).enumerate() {
            let y_pos = idx as u16 + 3; // Start after header
            queue!(stdout, MoveTo(2, y_pos))?;
            
            let line = &self.content[line_num];
            
            // Check if this line is within selection
            if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
                self.render_line_with_selection(line, y_pos, start, end, &mut stdout)?;
            } else {
                queue!(stdout, Print(self.truncate_line(line)))?;
            }
        }
        
        // Draw footer with scroll info
        let footer_y = (self.viewport_height + 3) as u16;
        queue!(
            stdout,
            MoveTo(0, footer_y),
            SetForegroundColor(Color::Cyan),
            Print("═".repeat(self.viewport_width + 4)),
            MoveTo(2, footer_y + 1),
            SetForegroundColor(Color::Green),
            Print(format!(
                "Lines {}-{} of {} | ↑↓: Scroll | PgUp/PgDn: Page | Tab: Next | Esc: Exit App",
                start_line + 1,
                end_line,
                self.content.len()
            )),
            ResetColor,
        )?;
        
        stdout.flush()?;
        Ok(())
    }
    
    fn render_line_with_selection(
        &self,
        line: &str,
        y_pos: u16,
        sel_start: (u16, u16),
        sel_end: (u16, u16),
        stdout: &mut impl Write,
    ) -> Result<()> {
        let line_str = self.truncate_line(line);
        
        // Normalize selection coordinates (ensure start is before end)
        let (start, end) = if sel_start.1 < sel_end.1 || 
            (sel_start.1 == sel_end.1 && sel_start.0 <= sel_end.0) {
            (sel_start, sel_end)
        } else {
            (sel_end, sel_start)
        };
        
        // Check if current line is within selection range
        if y_pos >= start.1 && y_pos <= end.1 {
            let x = 2u16; // Starting x position
            
            for (char_idx, ch) in line_str.chars().enumerate() {
                let char_x = x + char_idx as u16;
                
                let is_selected = if y_pos == start.1 && y_pos == end.1 {
                    // Selection on same line
                    char_x >= start.0 && char_x <= end.0
                } else if y_pos == start.1 {
                    // First line of selection
                    char_x >= start.0
                } else if y_pos == end.1 {
                    // Last line of selection
                    char_x <= end.0
                } else {
                    // Middle line - entire line selected
                    true
                };
                
                if is_selected {
                    queue!(
                        stdout,
                        SetBackgroundColor(Color::DarkGrey),
                        SetForegroundColor(Color::White),
                        Print(ch),
                        ResetColor
                    )?;
                } else {
                    queue!(stdout, Print(ch))?;
                }
            }
        } else {
            queue!(stdout, Print(line_str))?;
        }
        
        Ok(())
    }
    
    fn truncate_line(&self, line: &str) -> String {
        if line.len() > self.viewport_width {
            format!("{}…", &line[..self.viewport_width - 1])
        } else {
            line.to_string()
        }
    }
    
    pub fn handle_key_event(&mut self, key: event::KeyEvent) -> Result<Option<DebugViewerExit>> {
        match (key.modifiers, key.code) {
            // Copy selected text
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.copy_selection()?;
                return Ok(None); // Don't exit
            }
            // Select all
            (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                if !self.content.is_empty() {
                    self.selection_start = Some((2, 3));
                    let last_line_idx = self.content.len().saturating_sub(1);
                    let last_line_len = self.content[last_line_idx].len();
                    self.selection_end = Some((
                        2 + last_line_len.min(self.viewport_width) as u16,
                        3 + last_line_idx.min(self.viewport_height - 1) as u16
                    ));
                }
                return Ok(None);
            }
            // Navigation
            (_, KeyCode::Up) => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            (_, KeyCode::Down) => {
                if self.scroll_offset + self.viewport_height < self.content.len() {
                    self.scroll_offset += 1;
                }
            }
            (_, KeyCode::PageUp) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(self.viewport_height);
            }
            (_, KeyCode::PageDown) => {
                let max_scroll = self.content.len().saturating_sub(self.viewport_height);
                self.scroll_offset = (self.scroll_offset + self.viewport_height).min(max_scroll);
            }
            (_, KeyCode::Home) => {
                self.scroll_offset = 0;
            }
            (_, KeyCode::End) => {
                self.scroll_offset = self.content.len().saturating_sub(self.viewport_height);
            }
            // Tab to cycle to next screen
            (_, KeyCode::Tab) => {
                return Ok(Some(DebugViewerExit::Tab)); // Signal to cycle to next screen
            }
            // Esc to exit the entire app (consistent with other screens)
            (_, KeyCode::Esc) => {
                return Ok(Some(DebugViewerExit::Escape)); // Signal to exit app
            }
            _ => {}
        }
        
        Ok(None)
    }
    
    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Start selection
                self.is_selecting = true;
                self.selection_start = Some((mouse.column, mouse.row));
                self.selection_end = None;
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                // Update selection end
                if self.is_selecting {
                    self.selection_end = Some((mouse.column, mouse.row));
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                // Finish selection
                if self.is_selecting {
                    self.selection_end = Some((mouse.column, mouse.row));
                    self.is_selecting = false;
                }
            }
            MouseEventKind::ScrollUp => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            MouseEventKind::ScrollDown => {
                if self.scroll_offset + self.viewport_height < self.content.len() {
                    self.scroll_offset += 1;
                }
            }
            _ => {}
        }
        
        self.last_mouse_pos = (mouse.column, mouse.row);
        Ok(())
    }
    
    fn copy_selection(&self) -> Result<()> {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            // Normalize selection
            let (start, end) = if start.1 < end.1 || 
                (start.1 == end.1 && start.0 <= end.0) {
                (start, end)
            } else {
                (end, start)
            };
            
            let mut selected_text = String::new();
            
            // Calculate actual line indices from screen coordinates
            let start_line_idx = (start.1 as usize).saturating_sub(3) + self.scroll_offset;
            let end_line_idx = (end.1 as usize).saturating_sub(3) + self.scroll_offset;
            
            for line_idx in start_line_idx..=end_line_idx.min(self.content.len() - 1) {
                let line = &self.content[line_idx];
                
                if line_idx == start_line_idx && line_idx == end_line_idx {
                    // Single line selection
                    let start_col = (start.0 as usize).saturating_sub(2);
                    let end_col = ((end.0 as usize).saturating_sub(2)).min(line.len());
                    if start_col < line.len() {
                        selected_text.push_str(&line[start_col..end_col]);
                    }
                } else if line_idx == start_line_idx {
                    // First line
                    let start_col = (start.0 as usize).saturating_sub(2);
                    if start_col < line.len() {
                        selected_text.push_str(&line[start_col..]);
                        selected_text.push('\n');
                    }
                } else if line_idx == end_line_idx {
                    // Last line
                    let end_col = ((end.0 as usize).saturating_sub(2)).min(line.len());
                    selected_text.push_str(&line[..end_col]);
                } else {
                    // Middle lines
                    selected_text.push_str(line);
                    selected_text.push('\n');
                }
            }
            
            // Copy to clipboard
            if !selected_text.is_empty() {
                let mut clipboard = Clipboard::new()?;
                clipboard.set_text(selected_text)?;
                
                // Visual feedback could be added here
            }
        }
        
        Ok(())
    }
    
    pub fn run(&mut self) -> Result<DebugViewerExit> {
        // Enable mouse support
        crossterm::execute!(stdout(), crossterm::event::EnableMouseCapture)?;
        
        let exit_reason = loop {
            self.render()?;
            
            match event::read()? {
                Event::Key(key) => {
                    if let Some(exit) = self.handle_key_event(key)? {
                        break exit; // Exit requested
                    }
                }
                Event::Mouse(mouse) => {
                    self.handle_mouse_event(mouse)?;
                }
                Event::Resize(width, height) => {
                    self.viewport_width = width as usize - 4;
                    self.viewport_height = height as usize - 4;
                }
                _ => {}
            }
        };
        
        // Disable mouse support when exiting
        crossterm::execute!(stdout(), crossterm::event::DisableMouseCapture)?;
        
        Ok(exit_reason)
    }
}