// Excel-style grid editor for PDF text extraction
// Provides spreadsheet-like block selection and editing

use crossterm::event::KeyCode;
use std::cmp::{min, max};

#[derive(Debug, Clone)]
pub struct ExcelGrid {
    pub cells: Vec<Vec<char>>,
    pub cursor: (usize, usize),  // (col, row)
    pub selecting: bool,
    pub anchor: (usize, usize),   // selection start point
    pub width: usize,
    pub height: usize,
}

impl Default for ExcelGrid {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

impl ExcelGrid {
    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![vec![' '; width]; height];
        Self {
            cells,
            cursor: (0, 0),
            selecting: false,
            anchor: (0, 0),
            width,
            height,
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
        
        Self {
            cells,
            cursor: (0, 0),
            selecting: false,
            anchor: (0, 0),
            width,
            height: grid_height,
        }
    }
    
    /// Handle keyboard input
    pub fn handle_key(&mut self, key: KeyCode, shift_held: bool) {
        match key {
            // Toggle selection mode with Ctrl+V (visual block mode)
            KeyCode::Char('v') if !shift_held => {
                self.selecting = !self.selecting;
                if self.selecting {
                    self.anchor = self.cursor;
                }
            }
            
            // Movement keys
            KeyCode::Up => {
                if shift_held && !self.selecting {
                    self.selecting = true;
                    self.anchor = self.cursor;
                }
                self.cursor.1 = self.cursor.1.saturating_sub(1);
            }
            KeyCode::Down => {
                if shift_held && !self.selecting {
                    self.selecting = true;
                    self.anchor = self.cursor;
                }
                self.cursor.1 = (self.cursor.1 + 1).min(self.height - 1);
            }
            KeyCode::Left => {
                if shift_held && !self.selecting {
                    self.selecting = true;
                    self.anchor = self.cursor;
                }
                self.cursor.0 = self.cursor.0.saturating_sub(1);
            }
            KeyCode::Right => {
                if shift_held && !self.selecting {
                    self.selecting = true;
                    self.anchor = self.cursor;
                }
                self.cursor.0 = (self.cursor.0 + 1).min(self.width - 1);
            }
            
            // Escape cancels selection
            KeyCode::Esc => {
                self.selecting = false;
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
            
            // Character input
            KeyCode::Char(c) => {
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
        x >= x1 && x <= x2 && y >= y1 && y <= y2
    }
    
    /// Get the content as a string
    pub fn to_string(&self) -> String {
        self.cells
            .iter()
            .map(|row| row.iter().collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
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
            result.push(line.trim_end().to_string());
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