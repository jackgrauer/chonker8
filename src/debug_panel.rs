use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, SetBackgroundColor, SetForegroundColor, ResetColor},
    event::{MouseEvent, MouseEventKind, MouseButton},
};
use std::io::{self, Write};
use copypasta::{ClipboardContext, ClipboardProvider};

pub struct DebugPanel {
    pub logs: Vec<String>,
    pub scroll_offset: usize,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
    pub max_logs: usize,
    pub is_selecting: bool,
    pub last_bounds: Option<(u16, u16, u16, u16)>, // x, y, width, height
}

impl DebugPanel {
    pub fn new() -> Self {
        Self {
            logs: Vec::new(),
            scroll_offset: 0,
            selection_start: None,
            selection_end: None,
            max_logs: 1000,
            is_selecting: false,
            last_bounds: None,
        }
    }
    
    pub fn add_log(&mut self, msg: String) {
        self.logs.push(msg);
        if self.logs.len() > self.max_logs {
            self.logs.remove(0);
        }
    }
    
    pub fn render(&mut self, start_x: u16, start_y: u16, width: u16, height: u16) -> io::Result<()> {
        // Store bounds for mouse handling
        self.last_bounds = Some((start_x, start_y, width, height));
        let mut stdout = io::stdout();
        
        // No header - main app renders the header
        
        // Draw content area
        let content_start_y = start_y + 1;
        let content_height = height.saturating_sub(1);
        
        for y in 0..content_height {
            execute!(stdout, MoveTo(start_x, content_start_y + y))?;
            
            let log_idx = self.scroll_offset + y as usize;
            if log_idx < self.logs.len() {
                let log = &self.logs[log_idx];
                
                // Check if this line is selected
                let is_selected = if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
                    let (start, end) = if start <= end { (start, end) } else { (end, start) };
                    log_idx >= start && log_idx <= end
                } else {
                    false
                };
                
                if is_selected {
                    execute!(stdout, SetBackgroundColor(Color::DarkBlue))?;
                    execute!(stdout, SetForegroundColor(Color::White))?;
                }
                
                // Truncate or pad to width
                let display_str = if log.len() > width as usize {
                    &log[..width as usize]
                } else {
                    log
                };
                write!(stdout, "{:<width$}", display_str, width = width as usize)?;
                
                if is_selected {
                    execute!(stdout, ResetColor)?;
                }
            } else {
                // Empty line
                write!(stdout, "{:width$}", "", width = width as usize)?;
            }
        }
        
        stdout.flush()?;
        Ok(())
    }
    
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }
    
    pub fn scroll_down(&mut self, lines: usize) {
        let max_scroll = self.logs.len().saturating_sub(10);
        self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
    }
    
    pub fn select_all(&mut self) {
        if !self.logs.is_empty() {
            self.selection_start = Some(0);
            self.selection_end = Some(self.logs.len() - 1);
        }
    }
    
    pub fn copy_selection(&self) -> Option<String> {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let (start, end) = if start <= end { (start, end) } else { (end, start) };
            let selected_logs: Vec<String> = self.logs[start..=end].to_vec();
            Some(selected_logs.join("\n"))
        } else {
            None
        }
    }
    
    pub fn copy_to_clipboard(&self) -> Result<(), String> {
        if let Some(text) = self.copy_selection() {
            let mut ctx = ClipboardContext::new()
                .map_err(|e| format!("Clipboard error: {}", e))?;
            ctx.set_contents(text)
                .map_err(|e| format!("Clipboard error: {}", e))?;
            Ok(())
        } else {
            Err("No selection".to_string())
        }
    }
    
    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> bool {
        if let Some((start_x, start_y, width, height)) = self.last_bounds {
            // Check if mouse is within debug panel bounds
            let content_start_y = start_y + 1; // Skip header
            let content_height = height.saturating_sub(1);
            
            if mouse.column >= start_x && mouse.column < start_x + width &&
               mouse.row >= content_start_y && mouse.row < content_start_y + content_height {
                
                // Convert mouse position to log line index
                let relative_y = mouse.row - content_start_y;
                let log_idx = self.scroll_offset + relative_y as usize;
                
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        // Start selection
                        if log_idx < self.logs.len() {
                            self.selection_start = Some(log_idx);
                            self.selection_end = Some(log_idx);
                            self.is_selecting = true;
                            return true;
                        }
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        // Continue selection
                        if self.is_selecting && log_idx < self.logs.len() {
                            self.selection_end = Some(log_idx);
                            return true;
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        // End selection
                        if self.is_selecting {
                            if log_idx < self.logs.len() {
                                self.selection_end = Some(log_idx);
                            }
                            self.is_selecting = false;
                            return true;
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        self.scroll_up(3);
                        return true;
                    }
                    MouseEventKind::ScrollDown => {
                        self.scroll_down(3);
                        return true;
                    }
                    _ => {}
                }
            }
        }
        false
    }
    
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
        self.is_selecting = false;
    }
}