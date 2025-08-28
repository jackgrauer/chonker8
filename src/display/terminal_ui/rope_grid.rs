// RopeGrid - A grid implementation backed by the ropey rope data structure
// This provides efficient text editing for large documents while maintaining
// the same Grid API for compatibility

use ropey::Rope;
use arboard::Clipboard;
use std::sync::{Arc, Mutex};

/// Represents a rectangular region that has been modified
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DirtyRegion {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl DirtyRegion {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self { x, y, width, height }
    }
    
    /// Check if this region contains a point
    pub fn contains(&self, col: usize, row: usize) -> bool {
        col >= self.x && col < self.x + self.width &&
        row >= self.y && row < self.y + self.height
    }
    
    /// Merge with another region to create a bounding box
    pub fn merge(&self, other: &DirtyRegion) -> DirtyRegion {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);
        DirtyRegion::new(x, y, x2 - x, y2 - y)
    }
}

/// A grid backed by a rope data structure for efficient large text handling
pub struct RopeGrid {
    rope: Rope,
    pub cursor: (usize, usize),  // (col, row)
    pub selecting: bool,
    pub anchor: (usize, usize),   // selection start point
    clipboard: Option<Arc<Mutex<Clipboard>>>,
    status_message: Option<String>,
    // Search functionality
    pub search_query: String,
    search_results: Vec<(usize, usize)>,  // (col, row) positions
    search_current_index: usize,
    pub searching: bool,
    // Scrolling offsets
    pub scroll_x: usize,
    pub scroll_y: usize,
    // Cached for performance
    cached_width: usize,  // Maximum line width
    
    // Revision tracking and dirty regions
    revision: u64,  // Increments on each change
    dirty_regions: Vec<DirtyRegion>,  // Regions modified since last render
}

impl RopeGrid {
    /// Create a new RopeGrid
    pub fn new(width: usize, height: usize) -> Self {
        let mut rope = Rope::new();
        // Initialize with empty lines
        for _ in 0..height {
            rope.append(Rope::from_str("\n"));
        }
        
        let clipboard = Clipboard::new().ok().map(|c| Arc::new(Mutex::new(c)));
        
        Self {
            rope,
            cursor: (0, 0),
            selecting: false,
            anchor: (0, 0),
            clipboard,
            status_message: None,
            search_query: String::new(),
            search_results: Vec::new(),
            search_current_index: 0,
            searching: false,
            scroll_x: 0,
            scroll_y: 0,
            cached_width: width,
            revision: 0,
            dirty_regions: Vec::new(),
        }
    }

    /// Create from PDF text
    pub fn from_pdftext(text: &str, min_width: usize) -> Self {
        let rope = Rope::from_str(text);
        
        // Calculate the maximum line width
        let mut max_width = min_width;
        for line in rope.lines() {
            let line_len = line.len_chars();
            if line_len > max_width {
                max_width = line_len;
            }
        }
        
        let clipboard = Clipboard::new().ok().map(|c| Arc::new(Mutex::new(c)));
        
        Self {
            rope,
            cursor: (0, 0),
            selecting: false,
            anchor: (0, 0),
            clipboard,
            status_message: None,
            search_query: String::new(),
            search_results: Vec::new(),
            search_current_index: 0,
            searching: false,
            scroll_x: 0,
            scroll_y: 0,
            cached_width: max_width,
            revision: 0,
            dirty_regions: Vec::new(),
        }
    }

    /// Get the width of the grid (longest line)
    pub fn width(&self) -> usize {
        self.cached_width
    }

    /// Get the height of the grid (number of lines)
    pub fn height(&self) -> usize {
        self.rope.len_lines()
    }
    
    /// Get the current revision number
    pub fn revision(&self) -> u64 {
        self.revision
    }
    
    /// Mark a region as dirty (needs redrawing)
    pub fn mark_dirty(&mut self, x: usize, y: usize, width: usize, height: usize) {
        let new_region = DirtyRegion::new(x, y, width, height);
        
        // Try to merge with existing regions to minimize draw calls
        let mut merged = false;
        for region in &mut self.dirty_regions {
            // If regions are adjacent or overlapping, merge them
            if region.x <= new_region.x + new_region.width && 
                new_region.x <= region.x + region.width &&
                region.y <= new_region.y + new_region.height && 
                new_region.y <= region.y + region.height {
                *region = region.merge(&new_region);
                merged = true;
                break;
            }
        }
        
        if !merged {
            self.dirty_regions.push(new_region);
        }
    }
    
    /// Mark entire viewport as dirty
    pub fn mark_all_dirty(&mut self) {
        self.dirty_regions.clear();
        self.dirty_regions.push(DirtyRegion::new(0, 0, self.width(), self.height()));
    }
    
    /// Get dirty regions since last clear
    pub fn get_dirty_regions(&self) -> &[DirtyRegion] {
        &self.dirty_regions
    }
    
    /// Clear dirty regions (call after rendering)
    pub fn clear_dirty_regions(&mut self) {
        self.dirty_regions.clear();
    }
    
    /// Increment revision (marks a change)
    fn increment_revision(&mut self) {
        self.revision += 1;
    }

    /// Get a character at a specific position
    pub fn get_char(&self, col: usize, row: usize) -> char {
        if row >= self.height() {
            return ' ';
        }

        let line = self.rope.line(row);
        // Note: line.len_chars() includes the newline, so we need to handle that
        let line_len = if line.len_chars() > 0 && line.char(line.len_chars() - 1) == '\n' {
            line.len_chars() - 1
        } else {
            line.len_chars()
        };
        
        if col >= line_len {
            return ' ';
        }

        line.char(col)
    }

    /// Set a character at a specific position
    pub fn set_char(&mut self, col: usize, row: usize, ch: char) {
        if row >= self.height() {
            // Extend the rope with new lines if needed
            let old_height = self.height();
            while self.height() <= row {
                self.rope.append(Rope::from_str("\n"));
            }
            // Mark new lines as dirty
            self.mark_dirty(0, old_height, self.width(), row - old_height + 1);
        }

        let line_start = self.rope.line_to_char(row);
        let line = self.rope.line(row);
        let line_len = line.len_chars();
        
        if col >= line_len {
            // Extend the line with spaces if needed
            let spaces_needed = col - line_len + 1;
            let spaces: String = " ".repeat(spaces_needed);
            let insert_pos = if row + 1 < self.height() {
                self.rope.line_to_char(row + 1) - 1  // Before the newline
            } else {
                self.rope.len_chars()
            };
            self.rope.insert(insert_pos, &spaces);
            // Mark extended region as dirty
            self.mark_dirty(line_len, row, spaces_needed, 1);
        }

        // Replace the character
        let char_pos = line_start + col;
        if char_pos < self.rope.len_chars() {
            self.rope.remove(char_pos..char_pos + 1);
            self.rope.insert_char(char_pos, ch);
        }

        // Update cached width if needed
        let new_line_len = self.rope.line(row).len_chars();
        if new_line_len > self.cached_width {
            self.cached_width = new_line_len;
        }
        
        // Mark the character position as dirty
        self.mark_dirty(col, row, 1, 1);
        self.increment_revision();
    }

    /// Get a line as a string
    pub fn get_line(&self, row: usize) -> String {
        if row >= self.height() {
            return String::new();
        }

        let line = self.rope.line(row);
        line.to_string().trim_end_matches('\n').to_string()
    }

    /// Get text in a rectangular region
    pub fn get_text(&self, start_col: usize, start_row: usize, end_col: usize, end_row: usize) -> String {
        let mut result = String::new();
        
        for row in start_row..=end_row {
            if row >= self.height() {
                break;
            }
            
            let line = self.rope.line(row);
            let line_str = line.to_string();
            let line_chars: Vec<char> = line_str.chars().collect();
            
            let start = if row == start_row { start_col } else { 0 };
            let end = if row == end_row { end_col.min(line_chars.len()) } else { line_chars.len() };
            
            for col in start..end {
                if col < line_chars.len() && line_chars[col] != '\n' {
                    result.push(line_chars[col]);
                } else if col >= line_chars.len() {
                    result.push(' ');
                }
            }
            
            if row < end_row {
                result.push('\n');
            }
        }
        
        result
    }

    /// Insert text at cursor position
    pub fn insert_text(&mut self, text: &str) {
        let old_cursor = self.cursor;
        let line_start = self.rope.line_to_char(self.cursor.1);
        let char_pos = line_start + self.cursor.0;
        
        self.rope.insert(char_pos, text);
        
        // Update cursor position more accurately
        let text_lines: Vec<&str> = text.split('\n').collect();
        let inserted_lines = text_lines.len() - 1;
        
        if inserted_lines > 0 {
            // Text contains newlines
            self.mark_dirty(0, old_cursor.1, self.width(), inserted_lines + 1);
            
            self.cursor.1 += inserted_lines;
            // Cursor goes to position after last inserted character
            let last_line = text_lines.last().unwrap_or(&"");
            // If we inserted at middle of a line, cursor is at the inserted text position
            // If original line had text after cursor, that's now on the new line too
            self.cursor.0 = last_line.len();
        } else {
            // Single line insertion
            self.mark_dirty(old_cursor.0, old_cursor.1, text.len() + 10, 1);
            self.cursor.0 += text.len();
        }

        // Update cached width
        self.update_cached_width();
        self.increment_revision();
    }

    /// Delete character at cursor position
    pub fn delete_char(&mut self) {
        if self.cursor.1 >= self.height() {
            return;
        }

        let line_start = self.rope.line_to_char(self.cursor.1);
        let char_pos = line_start + self.cursor.0;
        
        if char_pos < self.rope.len_chars() {
            self.rope.remove(char_pos..char_pos + 1);
        }
    }

    /// Update the cached width by scanning all lines
    fn update_cached_width(&mut self) {
        let mut max_width = 0;
        for line in self.rope.lines() {
            let line_len = line.len_chars();
            if line_len > max_width {
                max_width = line_len;
            }
        }
        self.cached_width = max_width;
    }

    /// Convert to a string (for compatibility)
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }


    /// Ensure cursor is visible in viewport
    pub fn ensure_cursor_visible(&mut self, viewport_width: usize, viewport_height: usize) {
        // Horizontal scrolling
        if self.cursor.0 < self.scroll_x {
            self.scroll_x = self.cursor.0;
        } else if self.cursor.0 >= self.scroll_x + viewport_width {
            self.scroll_x = self.cursor.0.saturating_sub(viewport_width - 1);
        }

        // Vertical scrolling
        if self.cursor.1 < self.scroll_y {
            self.scroll_y = self.cursor.1;
        } else if self.cursor.1 >= self.scroll_y + viewport_height {
            self.scroll_y = self.cursor.1.saturating_sub(viewport_height - 1);
        }
    }

    /// Handle keyboard input with modifiers and viewport
    pub fn handle_key_with_modifiers_with_viewport(&mut self, key: crossterm::event::KeyCode, shift: bool, ctrl: bool, _alt: bool, viewport_width: usize, viewport_height: usize) {
        self.handle_key_with_modifiers(key, shift, ctrl, _alt);
        self.ensure_cursor_visible(viewport_width, viewport_height);
    }

    /// Handle keyboard input with modifiers
    pub fn handle_key_with_modifiers(&mut self, key: crossterm::event::KeyCode, shift: bool, ctrl: bool, _alt: bool) {
        use crossterm::event::KeyCode;

        // Handle selection mode
        if shift && !self.selecting {
            self.selecting = true;
            self.anchor = self.cursor;
        } else if !shift && self.selecting {
            self.selecting = false;
        }

        match key {
            KeyCode::Char(c) if !ctrl => {
                if self.selecting {
                    self.delete_selection();
                }
                self.set_char(self.cursor.0, self.cursor.1, c);
                self.cursor.0 += 1;
            }
            KeyCode::Char('c') if ctrl => {
                self.copy_to_clipboard();
            }
            KeyCode::Char('x') if ctrl => {
                self.cut_to_clipboard();
            }
            KeyCode::Char('v') if ctrl => {
                self.paste_from_clipboard();
            }
            KeyCode::Char('a') if ctrl => {
                self.selecting = true;
                self.anchor = (0, 0);
                self.cursor = (self.width(), self.height().saturating_sub(1));
            }
            KeyCode::Char('f') if ctrl => {
                self.start_search();
            }
            KeyCode::Backspace => {
                if self.selecting {
                    self.delete_selection();
                } else if self.cursor.0 > 0 {
                    self.cursor.0 -= 1;
                    self.delete_char();
                } else if self.cursor.1 > 0 {
                    // Move to end of previous line (excluding newline)
                    self.cursor.1 -= 1;
                    let line = self.rope.line(self.cursor.1);
                    let line_len = if line.len_chars() > 0 && line.char(line.len_chars() - 1) == '\n' {
                        line.len_chars() - 1
                    } else {
                        line.len_chars()
                    };
                    self.cursor.0 = line_len;
                    // Now delete the newline that was at the end of this line
                    self.delete_char();
                }
            }
            KeyCode::Delete => {
                if self.selecting {
                    self.delete_selection();
                } else {
                    self.delete_char();
                }
            }
            KeyCode::Enter => {
                if self.selecting {
                    self.delete_selection();
                }
                self.insert_text("\n");
            }
            KeyCode::Tab => {
                if self.selecting {
                    self.delete_selection();
                }
                self.insert_text("\t");
            }
            KeyCode::Esc => {
                if self.searching {
                    self.exit_search();
                } else if self.selecting {
                    self.clear_selection();
                }
            }
            KeyCode::Left => {
                if self.cursor.0 > 0 {
                    let old_cursor = self.cursor;
                    self.cursor.0 -= 1;
                    // Mark old and new cursor positions as dirty
                    self.mark_dirty(old_cursor.0, old_cursor.1, 1, 1);
                    self.mark_dirty(self.cursor.0, self.cursor.1, 1, 1);
                }
            }
            KeyCode::Right => {
                let old_cursor = self.cursor;
                self.cursor.0 += 1;
                // Mark old and new cursor positions as dirty
                self.mark_dirty(old_cursor.0, old_cursor.1, 1, 1);
                self.mark_dirty(self.cursor.0, self.cursor.1, 1, 1);
            }
            KeyCode::Up => {
                if self.cursor.1 > 0 {
                    let old_cursor = self.cursor;
                    self.cursor.1 -= 1;
                    // Clamp cursor to line length (excluding newline)
                    let line = self.rope.line(self.cursor.1);
                    let line_len = if line.len_chars() > 0 && line.char(line.len_chars() - 1) == '\n' {
                        line.len_chars() - 1
                    } else {
                        line.len_chars()
                    };
                    if self.cursor.0 > line_len {
                        self.cursor.0 = line_len;
                    }
                    // Mark old and new cursor positions as dirty
                    self.mark_dirty(old_cursor.0, old_cursor.1, 1, 1);
                    self.mark_dirty(self.cursor.0, self.cursor.1, 1, 1);
                }
            }
            KeyCode::Down => {
                if self.cursor.1 < self.height() - 1 {
                    let old_cursor = self.cursor;
                    self.cursor.1 += 1;
                    // Clamp cursor to line length (excluding newline)
                    let line = self.rope.line(self.cursor.1);
                    let line_len = if line.len_chars() > 0 && line.char(line.len_chars() - 1) == '\n' {
                        line.len_chars() - 1
                    } else {
                        line.len_chars()
                    };
                    if self.cursor.0 > line_len {
                        self.cursor.0 = line_len;
                    }
                    // Mark old and new cursor positions as dirty
                    self.mark_dirty(old_cursor.0, old_cursor.1, 1, 1);
                    self.mark_dirty(self.cursor.0, self.cursor.1, 1, 1);
                }
            }
            KeyCode::Home => {
                self.cursor.0 = 0;
            }
            KeyCode::End => {
                let line = self.rope.line(self.cursor.1);
                // Go to end of actual text, not including newline
                let line_len = if line.len_chars() > 0 && line.char(line.len_chars() - 1) == '\n' {
                    line.len_chars() - 1
                } else {
                    line.len_chars()
                };
                self.cursor.0 = line_len;
            }
            KeyCode::PageUp => {
                self.cursor.1 = self.cursor.1.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.cursor.1 = (self.cursor.1 + 10).min(self.height() - 1);
            }
            _ => {}
        }
    }

    /// Handle simple keyboard input
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode, shift_held: bool) {
        self.handle_key_with_modifiers(key, shift_held, false, false);
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selecting = false;
        self.anchor = self.cursor;
    }

    /// Delete selected text
    fn delete_selection(&mut self) {
        if !self.selecting {
            return;
        }

        let (start_col, start_row, end_col, end_row) = self.get_selection_bounds();
        
        // Delete the selected text
        for row in (start_row..=end_row).rev() {
            if row >= self.height() {
                continue;
            }
            
            let line_start = self.rope.line_to_char(row);
            let start = if row == start_row { start_col } else { 0 };
            let end = if row == end_row { end_col } else {
                self.rope.line(row).len_chars()
            };
            
            if start < end {
                let char_start = line_start + start;
                let char_end = line_start + end;
                self.rope.remove(char_start..char_end);
            }
        }
        
        self.cursor = (start_col, start_row);
        self.selecting = false;
        self.update_cached_width();
    }

    /// Copy to clipboard
    pub fn copy_to_clipboard(&mut self) {
        if let Some(text) = self.get_selected_text() {
            if let Some(clipboard) = &self.clipboard {
                if let Ok(mut clip) = clipboard.lock() {
                    let _ = clip.set_text(text);
                    self.status_message = Some("Copied to clipboard".to_string());
                }
            }
        }
    }

    /// Cut to clipboard
    pub fn cut_to_clipboard(&mut self) {
        if self.selecting {
            self.copy_to_clipboard();
            self.delete_selection();
        }
    }

    /// Paste from clipboard
    pub fn paste_from_clipboard(&mut self) {
        let text_to_paste = if let Some(clipboard) = &self.clipboard {
            if let Ok(mut clip) = clipboard.lock() {
                clip.get_text().ok()
            } else {
                None
            }
        } else {
            None
        };
        
        if let Some(text) = text_to_paste {
            if self.selecting {
                self.delete_selection();
            }
            self.insert_text(&text);
        }
    }

    /// Start search
    pub fn start_search(&mut self) {
        self.searching = true;
        self.search_query.clear();
        self.search_results.clear();
        self.search_current_index = 0;
    }

    /// Exit search
    pub fn exit_search(&mut self) {
        self.searching = false;
    }

    /// Perform search
    pub fn perform_search(&mut self) {
        if self.search_query.is_empty() {
            return;
        }

        self.search_results.clear();
        let query = self.search_query.to_lowercase();
        
        for row in 0..self.height() {
            let line = self.get_line(row).to_lowercase();
            let mut col = 0;
            
            while let Some(pos) = line[col..].find(&query) {
                self.search_results.push((col + pos, row));
                col += pos + 1;
            }
        }
        
        if !self.search_results.is_empty() {
            self.search_current_index = 0;
            let (col, row) = self.search_results[0];
            self.cursor = (col, row);
        }
    }

    /// Find next search result
    pub fn find_next(&mut self) {
        if !self.search_results.is_empty() {
            self.search_current_index = (self.search_current_index + 1) % self.search_results.len();
            let (col, row) = self.search_results[self.search_current_index];
            self.cursor = (col, row);
        }
    }

    /// Find previous search result  
    pub fn find_previous(&mut self) {
        if !self.search_results.is_empty() {
            if self.search_current_index == 0 {
                self.search_current_index = self.search_results.len() - 1;
            } else {
                self.search_current_index -= 1;
            }
            let (col, row) = self.search_results[self.search_current_index];
            self.cursor = (col, row);
        }
    }

    /// Check if position is a search match
    pub fn is_search_match(&self, col: usize, row: usize) -> bool {
        if self.search_query.is_empty() {
            return false;
        }
        
        self.search_results.iter().any(|&(c, r)| {
            r == row && col >= c && col < c + self.search_query.len()
        })
    }

    /// Check if position is selected
    pub fn is_selected(&self, x: usize, y: usize) -> bool {
        if !self.selecting {
            return false;
        }
        
        let (start_col, start_row, end_col, end_row) = self.get_selection_bounds();
        y >= start_row && y <= end_row && 
        ((y == start_row && x >= start_col) || y > start_row) &&
        ((y == end_row && x <= end_col) || y < end_row)
    }

    /// Handle mouse down
    pub fn handle_mouse_down(&mut self, col: usize, row: usize) {
        self.cursor = (col, row);
        self.selecting = true;
        self.anchor = (col, row);
    }

    /// Handle mouse down with viewport
    pub fn handle_mouse_down_with_viewport(&mut self, col: usize, row: usize, viewport_width: usize, viewport_height: usize) {
        self.handle_mouse_down(col, row);
        self.ensure_cursor_visible(viewport_width, viewport_height);
    }

    /// Handle mouse drag
    pub fn handle_mouse_drag(&mut self, col: usize, row: usize) {
        if self.selecting {
            self.cursor = (col, row);
        }
    }

    /// Handle mouse up
    pub fn handle_mouse_up(&mut self, _col: usize, _row: usize) {
        // Selection remains active after mouse up
    }

    /// Handle mouse up with viewport
    pub fn handle_mouse_up_with_viewport(&mut self, col: usize, row: usize, viewport_width: usize, viewport_height: usize) {
        self.handle_mouse_up(col, row);
        self.ensure_cursor_visible(viewport_width, viewport_height);
    }

    /// Get status message
    pub fn get_status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    /// Clear status message
    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }

    /// Copy selection to string
    pub fn copy_selection(&self) -> String {
        self.get_selected_text().unwrap_or_default()
    }

    /// Paste text at cursor
    pub fn paste_text(&mut self, text: &str) {
        if self.selecting {
            self.delete_selection();
        }
        self.insert_text(text);
    }

    /// Set viewport size (for compatibility)
    pub fn set_viewport_size(&mut self, _viewport_width: usize, _viewport_height: usize) {
        // Viewport size is handled dynamically in RopeGrid
    }

    /// Get selected text
    pub fn get_selected_text(&self) -> Option<String> {
        if !self.selecting {
            return None;
        }

        let (start_col, start_row, end_col, end_row) = self.get_selection_bounds();
        Some(self.get_text(start_col, start_row, end_col, end_row))
    }

    /// Get selection bounds
    pub fn get_selection_bounds(&self) -> (usize, usize, usize, usize) {
        let (x1, y1) = self.anchor;
        let (x2, y2) = self.cursor;
        
        if y1 < y2 || (y1 == y2 && x1 < x2) {
            (x1, y1, x2, y2)
        } else {
            (x2, y2, x1, y1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rope_grid_creation() {
        let grid = RopeGrid::new(80, 24);
        assert_eq!(grid.height(), 24);
    }

    #[test]
    fn test_char_operations() {
        let mut grid = RopeGrid::new(80, 24);
        grid.set_char(0, 0, 'H');
        grid.set_char(1, 0, 'i');
        assert_eq!(grid.get_char(0, 0), 'H');
        assert_eq!(grid.get_char(1, 0), 'i');
    }

    #[test]
    fn test_from_pdftext() {
        let text = "Line 1\nLine 2\nLine 3";
        let grid = RopeGrid::from_pdftext(text, 40);
        assert_eq!(grid.height(), 3);
        assert_eq!(grid.get_line(0), "Line 1");
        assert_eq!(grid.get_line(1), "Line 2");
        assert_eq!(grid.get_line(2), "Line 3");
    }
}