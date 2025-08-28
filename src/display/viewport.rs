// Viewport abstraction for managing separate screen regions
use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, SetForegroundColor, ResetColor},
    terminal::{Clear, ClearType},
};
use std::io::stdout;

/// Represents a bounded region of the terminal that can be rendered independently
#[derive(Debug, Clone)]
pub struct Viewport {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub name: String,
    /// Track if this viewport needs redrawing
    dirty: bool,
    /// Track if content has been initialized
    initialized: bool,
}

impl Viewport {
    pub fn new(name: &str, x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
            name: name.to_string(),
            dirty: true,
            initialized: false,
        }
    }

    /// Mark this viewport as needing a redraw
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Check if viewport needs redrawing
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark as clean after rendering
    pub fn mark_clean(&mut self) {
        self.dirty = false;
        self.initialized = true;
    }

    /// Clear only this viewport's area
    pub fn clear(&self) -> Result<()> {
        for y in self.y..self.y + self.height {
            execute!(
                stdout(),
                MoveTo(self.x, y),
                Clear(ClearType::UntilNewLine)
            )?;
        }
        Ok(())
    }

    /// Clear a specific line within the viewport
    pub fn clear_line(&self, line: u16) -> Result<()> {
        // Fix: Use >= to prevent off-by-one error
        if line >= self.height {
            return Ok(()); // Line is outside viewport, nothing to clear
        }
        
        execute!(
            stdout(),
            MoveTo(self.x, self.y + line),
            Clear(ClearType::UntilNewLine)
        )?;
        Ok(())
    }

    /// Move cursor to a position within this viewport
    pub fn move_to(&self, x: u16, y: u16) -> Result<()> {
        // Fix: Prevent arithmetic overflow with saturating operations
        let actual_x = self.x.saturating_add(x).min(self.x.saturating_add(self.width).saturating_sub(1));
        let actual_y = self.y.saturating_add(y).min(self.y.saturating_add(self.height).saturating_sub(1));
        execute!(stdout(), MoveTo(actual_x, actual_y))?;
        Ok(())
    }

    /// Check if a position is within this viewport
    pub fn contains(&self, x: u16, y: u16) -> bool {
        // Fix: Use saturating arithmetic to prevent overflow
        x >= self.x && x < self.x.saturating_add(self.width) &&
        y >= self.y && y < self.y.saturating_add(self.height)
    }

    /// Draw a border around this viewport
    pub fn draw_border(&self, color: Color) -> Result<()> {
        execute!(stdout(), SetForegroundColor(color))?;
        
        // Top border
        self.move_to(0, 0)?;
        print!("┌");
        for _ in 1..self.width - 1 {
            print!("─");
        }
        print!("┐");

        // Side borders
        for y in 1..self.height - 1 {
            self.move_to(0, y)?;
            print!("│");
            self.move_to(self.width - 1, y)?;
            print!("│");
        }

        // Bottom border
        self.move_to(0, self.height - 1)?;
        print!("└");
        for _ in 1..self.width - 1 {
            print!("─");
        }
        print!("┘");

        execute!(stdout(), ResetColor)?;
        Ok(())
    }
}

/// Manages multiple viewports and their relationships
pub struct ViewportManager {
    pub pdf_viewport: Viewport,
    pub text_viewport: Viewport,
    pub status_viewport: Viewport,
    /// Track if headers need redrawing
    headers_dirty: bool,
}

impl ViewportManager {
    pub fn new(terminal_width: u16, terminal_height: u16) -> Self {
        let split_x = terminal_width / 2;
        
        Self {
            pdf_viewport: Viewport::new(
                "PDF",
                0,
                1,  // Leave top row for header
                split_x,
                terminal_height - 2,  // Leave bottom row for status
            ),
            text_viewport: Viewport::new(
                "Text",
                split_x + 1,
                1,
                terminal_width - split_x - 1,
                terminal_height - 2,
            ),
            status_viewport: Viewport::new(
                "Status",
                0,
                terminal_height - 1,
                terminal_width,
                1,
            ),
            headers_dirty: true,
        }
    }

    /// Update viewport dimensions on terminal resize
    pub fn resize(&mut self, terminal_width: u16, terminal_height: u16) {
        let split_x = terminal_width / 2;
        
        self.pdf_viewport = Viewport::new("PDF", 0, 1, split_x, terminal_height - 2);
        self.text_viewport = Viewport::new("Text", split_x + 1, 1, terminal_width - split_x - 1, terminal_height - 2);
        self.status_viewport = Viewport::new("Status", 0, terminal_height - 1, terminal_width, 1);
        
        // Mark everything as dirty on resize
        self.pdf_viewport.mark_dirty();
        self.text_viewport.mark_dirty();
        self.status_viewport.mark_dirty();
        self.headers_dirty = true;
    }

    /// Check if headers need redrawing
    pub fn headers_need_redraw(&self) -> bool {
        self.headers_dirty
    }

    /// Mark headers as clean
    pub fn mark_headers_clean(&mut self) {
        self.headers_dirty = false;
    }

    /// Draw the vertical separator between panels
    pub fn draw_separator(&self) -> Result<()> {
        let split_x = self.text_viewport.x - 1;
        execute!(stdout(), SetForegroundColor(Color::DarkGrey))?;
        
        for y in 1..self.status_viewport.y {
            execute!(stdout(), MoveTo(split_x, y))?;
            print!("│");
        }
        
        execute!(stdout(), ResetColor)?;
        Ok(())
    }

    /// Only clear viewports that are dirty
    pub fn clear_dirty(&mut self) -> Result<()> {
        if self.pdf_viewport.is_dirty() && !self.pdf_viewport.initialized {
            self.pdf_viewport.clear()?;
        }
        if self.text_viewport.is_dirty() {
            self.text_viewport.clear()?;
        }
        if self.status_viewport.is_dirty() {
            self.status_viewport.clear()?;
        }
        Ok(())
    }
}