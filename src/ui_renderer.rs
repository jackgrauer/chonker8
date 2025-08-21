// Dynamic UI renderer that reads from hot-reloadable config
use crate::ui_config::UIConfig;
use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, stdout, Write};

pub struct UIRenderer {
    config: UIConfig,
    pdf_content: Vec<Vec<char>>,
    current_page: usize,
    total_pages: usize,
    scroll_offset: usize,
    cursor_x: usize,
    cursor_y: usize,
}

impl UIRenderer {
    pub fn new(config: UIConfig) -> Self {
        Self {
            config,
            pdf_content: vec![vec![' '; 80]; 24], // Default empty content
            current_page: 1,
            total_pages: 1,
            scroll_offset: 0,
            cursor_x: 0,
            cursor_y: 0,
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
    
    pub fn render(&self) -> Result<()> {
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
        match self.current_page {
            1 => self.pdf_content.clone(),
            2 => {
                // Create page 2 content
                let mut content = vec![vec![' '; 80]; 24];
                let lines = vec![
                    "╔══════════════════════════════════════╗",
                    "║  Chonker8.1 - Page 2 Demo           ║",
                    "╠══════════════════════════════════════╣",
                    "║                                      ║",
                    "║  This is page 2! Hot-reload works!  ║",
                    "║                                      ║",
                    "║  Features:                           ║",
                    "║    ✓ Multi-page navigation           ║",
                    "║    ✓ Live config reloading           ║",
                    "║    ✓ Crossterm TUI                   ║",
                    "║    ✓ Tab key navigation              ║",
                    "║                                      ║",
                    "║  Configuration files:                ║",
                    "║    - ui.toml (hot-reloadable)        ║",
                    "║    - Cargo.toml (build config)       ║",
                    "║                                      ║",
                    "║  Press Tab again to go back to P1!  ║",
                    "║                                      ║",
                    "║  Or use 'n' and 'p' for nav         ║",
                    "║                                      ║",
                    "╚══════════════════════════════════════╝",
                ];
                
                for (i, line) in lines.iter().enumerate() {
                    for (j, ch) in line.chars().enumerate() {
                        if j < 80 {
                            content[i][j] = ch;
                        }
                    }
                }
                content
            },
            _ => self.pdf_content.clone(),
        }
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
        
        // Left side: mode and file info
        let left_status = format!(" [{}] Page {}/{} ", 
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
}