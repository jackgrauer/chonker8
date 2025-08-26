// Excel-style grid editor for PDF text extraction
// Provides spreadsheet-like block selection and editing

use crossterm::event::KeyCode;
use std::cmp::{min, max};
use arboard::Clipboard;
use std::sync::{Arc, Mutex};

pub struct ExcelGrid {
    pub cells: Vec<Vec<char>>,
    pub cursor: (usize, usize),  // (col, row)
    pub selecting: bool,
    pub anchor: (usize, usize),   // selection start point
    pub width: usize,
    pub height: usize,
    clipboard: Option<Arc<Mutex<Clipboard>>>,
    status_message: Option<String>,
}

impl Clone for ExcelGrid {
    fn clone(&self) -> Self {
        // Create a new clipboard instance for the clone
        let clipboard = Clipboard::new().ok().map(|c| Arc::new(Mutex::new(c)));
        Self {
            cells: self.cells.clone(),
            cursor: self.cursor,
            selecting: self.selecting,
            anchor: self.anchor,
            width: self.width,
            height: self.height,
            clipboard,
            status_message: self.status_message.clone(),
        }
    }
}

impl std::fmt::Debug for ExcelGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExcelGrid")
            .field("cursor", &self.cursor)
            .field("selecting", &self.selecting)
            .field("anchor", &self.anchor)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("status_message", &self.status_message)
            .finish()
    }
}

impl Default for ExcelGrid {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMode {
    Character,
    Word,
    Line,
    Block,
    All,
}

impl ExcelGrid {
    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![vec![' '; width]; height];
        let clipboard = Clipboard::new().ok().map(|c| Arc::new(Mutex::new(c)));
        Self {
            cells,
            cursor: (0, 0),
            selecting: false,
            anchor: (0, 0),
            width,
            height,
            clipboard,
            status_message: None,
        }
    }
    
    /// Create grid from pdftotext output
    pub fn from_pdftext(text: &str, width: usize) -> Self {
        let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
        let height = lines.len().max(24);
        
        let mut cells = Vec::with_capacity(height);
        for line in lines.iter() {
            let mut row: Vec<char> = line.chars().collect();
            row.resize(width, ' ');
            cells.push(row);
        }
        
        // Ensure minimum height
        while cells.len() < height {
            cells.push(vec![' '; width]);
        }
        
        let grid_height = cells.len();
        let clipboard = Clipboard::new().ok().map(|c| Arc::new(Mutex::new(c)));
        
        Self {
            cells,
            cursor: (0, 0),
            selecting: false,
            anchor: (0, 0),
            width,
            height: grid_height,
            clipboard,
            status_message: None,
        }
    }
    
    /// Handle keyboard input with modifiers
    pub fn handle_key_with_modifiers(&mut self, key: KeyCode, shift: bool, ctrl: bool, _alt: bool) {
        match key {
            // Basic cut/copy/paste only
            KeyCode::Char('c') if ctrl => {
                self.copy_to_clipboard();
            }
            
            KeyCode::Char('x') if ctrl => {
                self.cut_to_clipboard();
            }
            
            KeyCode::Char('v') if ctrl => {
                self.paste_from_clipboard();
            }
            
            // Simple arrow key movement
            KeyCode::Up => {
                if shift && !self.selecting {
                    self.selecting = true;
                    self.anchor = self.cursor;
                    self.status_message = Some(format!("Selection started at ({},{})", self.anchor.0, self.anchor.1));
                }
                self.cursor.1 = self.cursor.1.saturating_sub(1);
                if shift && self.selecting {
                    let (x1, y1, x2, y2) = self.get_selection_bounds();
                    self.status_message = Some(format!("Selecting ({},{}) to ({},{})", x1, y1, x2, y2));
                } else if !shift {
                    self.selecting = false;
                }
            }
            KeyCode::Down => {
                if shift && !self.selecting {
                    self.selecting = true;
                    self.anchor = self.cursor;
                    self.status_message = Some(format!("Selection started at ({},{})", self.anchor.0, self.anchor.1));
                }
                self.cursor.1 = (self.cursor.1 + 1).min(self.height - 1);
                if shift && self.selecting {
                    let (x1, y1, x2, y2) = self.get_selection_bounds();
                    self.status_message = Some(format!("Selecting ({},{}) to ({},{})", x1, y1, x2, y2));
                } else if !shift {
                    self.selecting = false;
                }
            }
            KeyCode::Left => {
                if shift && !self.selecting {
                    self.selecting = true;
                    self.anchor = self.cursor;
                    self.status_message = Some(format!("Selection started at ({},{})", self.anchor.0, self.anchor.1));
                }
                self.cursor.0 = self.cursor.0.saturating_sub(1);
                if shift && self.selecting {
                    let (x1, y1, x2, y2) = self.get_selection_bounds();
                    self.status_message = Some(format!("Selecting ({},{}) to ({},{})", x1, y1, x2, y2));
                } else if !shift {
                    self.selecting = false;
                }
            }
            KeyCode::Right => {
                if shift && !self.selecting {
                    self.selecting = true;
                    self.anchor = self.cursor;
                    self.status_message = Some(format!("Selection started at ({},{})", self.anchor.0, self.anchor.1));
                }
                self.cursor.0 = (self.cursor.0 + 1).min(self.width - 1);
                if shift && self.selecting {
                    let (x1, y1, x2, y2) = self.get_selection_bounds();
                    self.status_message = Some(format!("Selecting ({},{}) to ({},{})", x1, y1, x2, y2));
                } else if !shift {
                    self.selecting = false;
                }
            }
            
            // Escape cancels selection
            KeyCode::Esc => {
                self.selecting = false;
                self.status_message = Some("Selection cancelled".to_string());
            }
            
            // Delete/Backspace
            KeyCode::Delete => {
                if self.selecting {
                    self.clear_selection();
                } else {
                    self.cells[self.cursor.1][self.cursor.0] = ' ';
                }
            }
            KeyCode::Backspace => {
                if self.selecting {
                    self.clear_selection();
                } else if self.cursor.0 > 0 {
                    self.cursor.0 -= 1;
                    self.cells[self.cursor.1][self.cursor.0] = ' ';
                }
            }
            
            // Character input - handle both ctrl shortcuts and regular typing
            KeyCode::Char(c) if !ctrl => {
                // Regular typing (not a ctrl shortcut)
                if self.selecting {
                    // Replace all characters in selected block
                    let (x1, y1, x2, y2) = self.get_selection_bounds();
                    for y in y1..=y2 {
                        for x in x1..=x2 {
                            if y < self.cells.len() && x < self.cells[y].len() {
                                self.cells[y][x] = c;
                            }
                        }
                    }
                    self.selecting = false;
                    self.status_message = Some(format!("Replaced selection with '{}'", c));
                } else {
                    // Type at cursor
                    if self.cursor.1 < self.cells.len() && self.cursor.0 < self.cells[self.cursor.1].len() {
                        self.cells[self.cursor.1][self.cursor.0] = c;
                        // Move cursor right after typing
                        self.cursor.0 = (self.cursor.0 + 1).min(self.width - 1);
                    }
                }
            }
            
            // Home/End for line navigation
            KeyCode::Home => {
                self.cursor.0 = 0;
            }
            KeyCode::End => {
                // Find last non-space character in current line
                if self.cursor.1 < self.cells.len() {
                    let line = &self.cells[self.cursor.1];
                    for i in (0..self.width).rev() {
                        if line[i] != ' ' {
                            self.cursor.0 = (i + 1).min(self.width - 1);
                            return;
                        }
                    }
                    self.cursor.0 = 0;
                }
            }
            
            // Enter key - move to next line
            KeyCode::Enter => {
                self.cursor.1 = (self.cursor.1 + 1).min(self.height - 1);
                self.cursor.0 = 0;
            }
            
            // Page Up/Down
            KeyCode::PageUp => {
                self.cursor.1 = self.cursor.1.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.cursor.1 = (self.cursor.1 + 10).min(self.height - 1);
            }
            
            _ => {}
        }
    }
    
    /// Clear the selected block
    pub fn clear_selection(&mut self) {
        let (x1, y1, x2, y2) = self.get_selection_bounds();
        for y in y1..=y2 {
            for x in x1..=x2 {
                if y < self.cells.len() && x < self.cells[y].len() {
                    self.cells[y][x] = ' ';
                }
            }
        }
        self.selecting = false;
    }
    
    /// Get normalized selection bounds (x1, y1, x2, y2)
    pub fn get_selection_bounds(&self) -> (usize, usize, usize, usize) {
        (
            min(self.anchor.0, self.cursor.0),
            min(self.anchor.1, self.cursor.1),
            max(self.anchor.0, self.cursor.0),
            max(self.anchor.1, self.cursor.1),
        )
    }
    
    /// Check if a position is within the selection
    pub fn is_selected(&self, x: usize, y: usize) -> bool {
        if !self.selecting {
            return false;
        }
        let (x1, y1, x2, y2) = self.get_selection_bounds();
        let selected = x >= x1 && x <= x2 && y >= y1 && y <= y2;
        selected
    }
    
    /// Get the content as a string
    pub fn to_string(&self) -> String {
        self.cells
            .iter()
            .map(|row| row.iter().collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    /// Handle keyboard input (backward compatibility wrapper)
    pub fn handle_key(&mut self, key: KeyCode, shift_held: bool) {
        self.handle_key_with_modifiers(key, shift_held, false, false);
    }
    
    
    /// Copy selection to system clipboard
    pub fn copy_to_clipboard(&mut self) {
        if !self.selecting {
            self.status_message = Some("ERROR: No selection to copy".to_string());
            return;
        }
        
        let text = self.copy_selection();
        let text_preview = if text.len() > 20 {
            format!("{}...", text.chars().take(20).collect::<String>())
        } else {
            text.clone()
        };
        
        if let Some(clipboard) = self.clipboard.clone() {
            if let Ok(mut clip) = clipboard.lock() {
                match clip.set_text(&text) {
                    Ok(_) => {
                        self.status_message = Some(format!("COPIED: {} chars [{}]", text.len(), text_preview));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("COPY FAILED: {}", e));
                    }
                }
            } else {
                self.status_message = Some("ERROR: Clipboard locked".to_string());
            }
        } else {
            self.status_message = Some("ERROR: No clipboard".to_string());
        }
    }
    
    /// Cut selection to clipboard
    pub fn cut_to_clipboard(&mut self) {
        if !self.selecting {
            self.status_message = Some("ERROR: No selection to cut".to_string());
            return;
        }
        
        // Copy first
        let text = self.copy_selection();
        let chars_count = text.len();
        
        if let Some(clipboard) = self.clipboard.clone() {
            if let Ok(mut clip) = clipboard.lock() {
                match clip.set_text(&text) {
                    Ok(_) => {
                        // Clear after successful copy
                        self.clear_selection();
                        self.status_message = Some(format!("CUT: {} chars removed", chars_count));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("CUT FAILED: {}", e));
                    }
                }
            } else {
                self.status_message = Some("ERROR: Clipboard locked".to_string());
            }
        } else {
            self.status_message = Some("ERROR: No clipboard".to_string());
        }
    }
    
    /// Paste from system clipboard
    pub fn paste_from_clipboard(&mut self) {
        if let Some(clipboard) = self.clipboard.clone() {
            if let Ok(mut clip) = clipboard.lock() {
                match clip.get_text() {
                    Ok(text) => {
                        self.paste_text(&text);
                        self.status_message = Some(format!("✓ Pasted {} chars", text.len()));
                    }
                    Err(_) => {
                        self.status_message = Some("Nothing to paste".to_string());
                    }
                }
            } else {
                self.status_message = Some("Clipboard locked".to_string());
            }
        } else {
            self.status_message = Some("No clipboard available".to_string());
        }
    }
    
    
    /// Get current status message
    pub fn get_status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }
    
    /// Handle mouse events for selection
    pub fn handle_mouse_down(&mut self, col: usize, row: usize) {
        // Start selection at clicked position
        self.cursor = (col.min(self.width - 1), row.min(self.height - 1));
        self.anchor = self.cursor;
        self.selecting = false;  // Will become true on drag
        self.status_message = Some(format!("Click at ({},{})", col, row));
    }
    
    /// Handle mouse drag for selection
    pub fn handle_mouse_drag(&mut self, col: usize, row: usize) {
        // Update cursor position and enable selection
        self.cursor = (col.min(self.width - 1), row.min(self.height - 1));
        if !self.selecting && (self.cursor != self.anchor) {
            self.selecting = true;
        }
        
        if self.selecting {
            let (x1, y1, x2, y2) = self.get_selection_bounds();
            self.status_message = Some(format!("Mouse selecting ({},{}) to ({},{})", x1, y1, x2, y2));
        }
    }
    
    /// Handle mouse release
    pub fn handle_mouse_up(&mut self, col: usize, row: usize) {
        self.cursor = (col.min(self.width - 1), row.min(self.height - 1));
        if self.selecting {
            let (x1, y1, x2, y2) = self.get_selection_bounds();
            self.status_message = Some(format!("Selected ({},{}) to ({},{})", x1, y1, x2, y2));
        }
    }
    
    /// Clear status message
    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }
    
    /// Insert text at cursor position
    pub fn insert_text(&mut self, text: &str) {
        for ch in text.chars() {
            if ch == '\n' {
                // Move to next line
                self.cursor.1 = (self.cursor.1 + 1).min(self.height - 1);
                self.cursor.0 = 0;
            } else if ch == '\t' {
                // Tab moves 4 spaces
                self.cursor.0 = (self.cursor.0 + 4).min(self.width - 1);
            } else {
                // Insert character
                if self.cursor.1 < self.cells.len() && self.cursor.0 < self.cells[self.cursor.1].len() {
                    self.cells[self.cursor.1][self.cursor.0] = ch;
                    self.cursor.0 = (self.cursor.0 + 1).min(self.width - 1);
                }
            }
        }
    }
    
    /// Copy selected text to string
    pub fn copy_selection(&self) -> String {
        if !self.selecting {
            return String::new();
        }
        
        let (x1, y1, x2, y2) = self.get_selection_bounds();
        let mut result = Vec::new();
        
        for y in y1..=y2 {
            let mut line = String::new();
            for x in x1..=x2 {
                if y < self.cells.len() && x < self.cells[y].len() {
                    line.push(self.cells[y][x]);
                }
            }
            // Don't trim if it's all spaces - might be intentional
            result.push(line);
        }
        
        result.join("\n")
    }
    
    /// Replace selection with text
    pub fn paste_text(&mut self, text: &str) {
        if self.selecting {
            self.clear_selection();
        }
        self.insert_text(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_from_pdftext() {
        let text = "Hello World\nThis is a test\nLine 3";
        let grid = ExcelGrid::from_pdftext(text, 80);
        
        assert_eq!(grid.cells[0][0], 'H');
        assert_eq!(grid.cells[0][5], ' ');
        assert_eq!(grid.cells[0][6], 'W');
        assert_eq!(grid.cells[1][0], 'T');
        assert_eq!(grid.cells[2][0], 'L');
    }
    
    #[test]
    fn test_cursor_movement() {
        let mut grid = ExcelGrid::new(80, 24);
        
        grid.handle_key(KeyCode::Right, false);
        assert_eq!(grid.cursor, (1, 0));
        
        grid.handle_key(KeyCode::Down, false);
        assert_eq!(grid.cursor, (1, 1));
        
        grid.handle_key(KeyCode::Left, false);
        assert_eq!(grid.cursor, (0, 1));
        
        grid.handle_key(KeyCode::Up, false);
        assert_eq!(grid.cursor, (0, 0));
    }
    
    #[test]
    fn test_typing() {
        let mut grid = ExcelGrid::new(80, 24);
        
        grid.handle_key(KeyCode::Char('H'), false);
        assert_eq!(grid.cells[0][0], 'H');
        assert_eq!(grid.cursor, (1, 0));
        
        grid.handle_key(KeyCode::Char('i'), false);
        assert_eq!(grid.cells[0][1], 'i');
        assert_eq!(grid.cursor, (2, 0));
    }
    
    #[test]
    fn test_block_selection() {
        let mut grid = ExcelGrid::new(80, 24);
        
        // Start selection with Shift+Right
        grid.handle_key(KeyCode::Right, true);
        assert!(grid.selecting);
        assert_eq!(grid.anchor, (0, 0));
        assert_eq!(grid.cursor, (1, 0));
        
        // Extend selection
        grid.handle_key(KeyCode::Down, true);
        assert_eq!(grid.cursor, (1, 1));
        
        // Type to replace block
        grid.handle_key(KeyCode::Char('X'), false);
        assert_eq!(grid.cells[0][0], 'X');
        assert_eq!(grid.cells[0][1], 'X');
        assert_eq!(grid.cells[1][0], 'X');
        assert_eq!(grid.cells[1][1], 'X');
        assert!(!grid.selecting);
    }
    
    #[test]
    fn test_selection_bounds() {
        let mut grid = ExcelGrid::new(80, 24);
        grid.cursor = (5, 5);
        grid.anchor = (2, 2);
        grid.selecting = true;
        
        let (x1, y1, x2, y2) = grid.get_selection_bounds();
        assert_eq!((x1, y1, x2, y2), (2, 2, 5, 5));
        
        // Test reverse selection
        grid.cursor = (1, 1);
        let (x1, y1, x2, y2) = grid.get_selection_bounds();
        assert_eq!((x1, y1, x2, y2), (1, 1, 2, 2));
    }
}